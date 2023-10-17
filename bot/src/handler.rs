use std::{collections::HashMap, error::Error, future::IntoFuture, marker::PhantomData};

use sec_store::repository::RecordsRepository;
use teloxide::{
    dispatching::{
        dialogue::{GetChatId, InMemStorage},
        DpHandlerDescription, HandlerExt, UpdateFilterExt,
    },
    dptree, filter_command,
    macros::BotCommands,
    payloads::SendMessageSetters,
    prelude::{DependencyMap, Handler},
    requests::Requester,
    types::{
        CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode, Update,
        UserId,
    },
    Bot,
};
use tokio::{sync::RwLock, task::JoinSet};

use crate::{dialogues::hello::HelloDialogue, user_repo_factory::RepositoriesFactory};
use crate::{
    dialogues_controller::{self, DialInteraction, DialogueController},
    stated_dialogues::{MessageFormat, MessageId},
};
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub enum BotState {
    #[default]
    Default,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Remove and initialize dialog")]
    Reset,
    #[command(description = "Cancel dialog action")]
    Cancel,
}

pub struct BotContext<F: RepositoriesFactory<R>, R: RecordsRepository> {
    pub factory: F,
    pub dial_ctxs: HashMap<UserId, DialogueController>,
    //
    phantom: PhantomData<R>,
}
impl<F, R> BotContext<F, R>
where
    F: RepositoriesFactory<R>,
    R: RecordsRepository,
{
    pub fn new(factory: F) -> Self {
        BotContext {
            factory,
            dial_ctxs: HashMap::new(),
            phantom: PhantomData,
        }
    }
}

type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type HandlerResult = AnyResult<()>;

pub fn build_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    let commands_handler = filter_command::<Command, _>()
        .branch(dptree::case![Command::Reset].endpoint(handle_reset_command::<F, R>))
        .endpoint(handle_command::<F, R>);

    let messages_hanler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<BotState>, BotState>()
        .branch(commands_handler)
        .endpoint(main_state_handler::<F, R>);

    let callbacks_hanlder = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<BotState>, BotState>()
        .endpoint(default_callback_handler::<F, R>);

    dptree::entry()
        .branch(messages_hanler)
        .branch(callbacks_hanlder)
}

async fn main_state_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
    bot: Bot,
    msg: Message,
    context: Arc<RwLock<BotContext<F, R>>>,
) -> HandlerResult {
    log::debug!(
        "Handling message. chat_id={} from={:?}",
        msg.chat.id,
        msg.from().map(|f| f.id)
    );

    let user_id = msg.from().unwrap().id;
    handle_interaction(
        &user_id,
        &bot,
        context,
        DialInteraction::Message(msg.into()),
    )
    .await
}

async fn default_callback_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
    bot: Bot,
    query: CallbackQuery,
    context: Arc<RwLock<BotContext<F, R>>>,
) -> HandlerResult {
    log::debug!(
        "Callback: called, chat_id: {:?}; from: {:?}",
        query.chat_id(),
        query.from.id
    );

    let user_id = query.from.id;

    log::debug!("Callback ({user_id}): Handling \"{:?}\"", query.data);
    handle_interaction(
        &user_id,
        &bot,
        context,
        DialInteraction::Select(query.into()),
    )
    .await
}

async fn handle_reset_command<F: RepositoriesFactory<R>, R: RecordsRepository>(
    bot: Bot,
    msg: Message,
    context: Arc<RwLock<BotContext<F, R>>>,
) -> HandlerResult {
    log::debug!(
        "Handling reset command. chat_id={} from={:?}",
        msg.chat.id,
        msg.from().map(|f| f.id)
    );
    let user_id = msg.from().unwrap().id;
    if let Some(old_controller) = context.write().await.dial_ctxs.remove(&user_id) {
        process_ctx_results(user_id, old_controller.shutdown()?, &bot).await?;
    }

    handle_interaction(
        &user_id,
        &bot,
        context,
        DialInteraction::Command(msg.clone().into()),
    )
    .await
}

async fn handle_command<F: RepositoriesFactory<R>, R: RecordsRepository>(
    bot: Bot,
    msg: Message,
    context: Arc<RwLock<BotContext<F, R>>>,
) -> HandlerResult {
    log::debug!(
        "Handling {:?} command. chat_id={} from={:?}",
        msg.text(),
        msg.chat.id,
        msg.from().map(|f| f.id)
    );
    let user_id = msg.from().unwrap().id;

    handle_interaction(
        &user_id,
        &bot,
        context,
        DialInteraction::Command(msg.clone().into()),
    )
    .await
}

async fn handle_interaction<F: RepositoriesFactory<R>, R: RecordsRepository>(
    user_id: &UserId,
    bot: &Bot,
    context: Arc<RwLock<BotContext<F, R>>>,
    interaction: DialInteraction,
) -> HandlerResult {
    let dial_controller = context.write().await.dial_ctxs.remove(user_id);

    let (controller, results) = match dial_controller {
        Some(controller) => controller.handle(interaction),
        None => {
            let (controller, results) =
                create_dial_controller::<F, R>(context.read().await.factory.clone(), user_id)?;
            controller
                .handle(interaction)
                .map(|(controller, handle_results)| {
                    (
                        controller,
                        results.into_iter().chain(handle_results).collect(),
                    )
                })
        }
    }?;

    let sent_msg_ids = process_ctx_results(*user_id, results, bot).await?;
    if let Some(mut controller) = controller {
        controller.remember_sent_messages(sent_msg_ids);
        context.write().await.dial_ctxs.insert(*user_id, controller);
    } else {
        let (mut controller, results) =
            create_dial_controller(context.read().await.factory.clone(), user_id)?;
        let sent_msg_ids = process_ctx_results(*user_id, results, bot).await?;

        controller.remember_sent_messages(sent_msg_ids);
        context.write().await.dial_ctxs.insert(*user_id, controller);
    }
    Ok(())
}

fn create_dial_controller<F: RepositoriesFactory<R>, R: RecordsRepository>(
    factory: F,
    user_id: &UserId,
) -> AnyResult<(DialogueController, Vec<dialogues_controller::CtxResult>)> {
    let context = HelloDialogue::<F, R>::new((*user_id).into(), factory);
    Ok(DialogueController::create(context)?)
}

async fn process_ctx_results(
    user_id: UserId,
    ctx_results: Vec<dialogues_controller::CtxResult>,
    bot: &Bot,
) -> AnyResult<Vec<MessageId>> {
    log::debug!(
        "Results processing ({user_id}): executing {} results...",
        ctx_results.len()
    );

    let mut sent_msg_ids: Vec<MessageId> = vec![];
    for ctx_result in ctx_results {
        match ctx_result {
            dialogues_controller::CtxResult::Messages(messages) => {
                log::debug!(
                    "Results processing ({user_id}): sending {} messages",
                    messages.len()
                );
                for msg in messages {
                    let send_request = bot.send_message(user_id, msg.text());
                    let send_request = match msg.format {
                        MessageFormat::Html => send_request.parse_mode(ParseMode::Html),
                        MessageFormat::Text => send_request,
                    };

                    send_request
                        .await
                        .map(|msg| sent_msg_ids.push(msg.id.into()))?;
                }
            }
            dialogues_controller::CtxResult::Buttons(msg, selector) => {
                log::debug!("Results processing ({user_id}): sending keyboard");
                let markup = InlineKeyboardMarkup::new(selector.into_iter().map(|buttons_row| {
                    buttons_row
                        .into_iter()
                        .map(|(payload, text)| InlineKeyboardButton::callback(text, payload))
                        .collect::<Vec<InlineKeyboardButton>>()
                }));
                let send_message = bot.send_message(user_id, msg.text());
                let send_message = match msg.format {
                    MessageFormat::Html => send_message.parse_mode(ParseMode::Html),
                    MessageFormat::Text => send_message,
                };

                let sent_message = send_message.reply_markup(markup).await?;
                sent_msg_ids.push(sent_message.id.into());
            }
            dialogues_controller::CtxResult::RemoveMessages(messages_ids) => {
                log::debug!(
                    "Results processing ({user_id}): removing {} messages",
                    messages_ids.len()
                );

                let mut set = JoinSet::new();
                messages_ids
                    .into_iter()
                    .map(|msg_id| bot.delete_message(user_id, msg_id.into()).into_future())
                    .for_each(|future| {
                        set.spawn(future);
                    });

                while let Some(res) = set.join_next().await {
                    if let Err(err) = res? {
                        log::error!("Failed message deletion: {}", err);
                    }
                }
            }
        }
    }
    log::debug!("Results processing ({user_id}): all results processed");
    Ok(sent_msg_ids)
}
