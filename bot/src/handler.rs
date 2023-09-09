use std::{collections::HashMap, error::Error, ops::Deref};

use sec_store::repository::RecordsRepository;
use teloxide::{
    dispatching::{
        dialogue::{self, GetChatId, InMemStorage},
        DpHandlerDescription, HandlerExt, UpdateFilterExt,
    },
    dptree,
    payloads::SendMessageSetters,
    prelude::{DependencyMap, Dialogue, Handler},
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, Update, UserId},
    Bot,
};
use tokio::sync::RwLock;

use crate::{
    stated_dialogues::{hello::HelloDialogue, CtxResult, DialContext, DialogueId},
    user_repo_factory::RepositoriesFactory,
};
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    MainState,
    CreateRepoState,
    CreateRepoStateEnterPass,
}

pub struct BotContext<T: RecordsRepository> {
    pub factory: Arc<Box<dyn RepositoriesFactory<T> + 'static + Sync + Send>>,
    pub dial_ctxs: HashMap<UserId, RwLock<Box<dyn DialContext + Sync + Send>>>,
}

pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

async fn initialize_dialog_ctx<T: RecordsRepository + 'static>(
    context: Arc<RwLock<BotContext<T>>>,
    user_id: UserId,
) -> Vec<CtxResult> {
    log::debug!("Creating new dialogue with {user_id}...");
    let mut wctx = context.write().await;
    let factory = wctx.factory.clone();
    let mut dial = HelloDialogue::<T>::new(DialogueId(user_id.to_string()), factory);
    let result = dial.init().expect("Failed dialogue initialize");

    wctx.dial_ctxs.insert(user_id, RwLock::new(Box::new(dial)));
    log::debug!("New dialogue with {user_id} created!");

    return vec![result];
}

async fn process_ctx_results<T: RecordsRepository + 'static>(
    user_id: UserId,
    context: Arc<RwLock<BotContext<T>>>,
    bot: Bot,
    ctx_results: Vec<CtxResult>,
) -> HandlerResult {
    let mut new_ctx_results: Vec<CtxResult> = vec![];
    let mut buckets: Vec<Vec<CtxResult>> = vec![ctx_results];

    while buckets.len() > 0 {
        let mut bucket = buckets.remove(0);
        while bucket.len() != 0 {
            match &bucket[0] {
                CtxResult::NewCtx(_) => break,
                _ => new_ctx_results.push(bucket.remove(0)),
            }
        }
        if bucket.is_empty() {
            break;
        }

        let mut new_bucket: Vec<CtxResult> = vec![];
        match bucket.remove(0) {
            CtxResult::NewCtx(new_ctx) => {
                if let Some(old_ctx) = context
                    .write()
                    .await
                    .dial_ctxs
                    .insert(user_id, RwLock::new(new_ctx))
                {
                    new_bucket.push(
                        old_ctx
                            .write()
                            .await
                            .shutdown()
                            .expect(format!("Failed ctx shutdown for {}", user_id).as_str()),
                    );
                }
                log::debug!("New ctx intialization for {user_id}");
                new_bucket.push(
                    context
                        .read()
                        .await
                        .dial_ctxs
                        .get(&user_id)
                        .unwrap()
                        .write()
                        .await
                        .init()
                        .expect(format!("Failed new context initialization {}", user_id).as_str()),
                );
            }
            _ => unreachable!(),
        }
        new_bucket.extend(bucket);
        buckets.push(new_bucket);
    }

    for ctx_result in new_ctx_results {
        match ctx_result {
            CtxResult::Messages(messages) => {
                for msg in messages {
                    bot.send_message(user_id, msg).await?;
                }
            }
            CtxResult::Buttons(msg, selector) => {
                let markup = InlineKeyboardMarkup::new(selector.into_iter().map(|buttons_row| {
                    buttons_row
                        .into_iter()
                        .map(|(payload, text)| InlineKeyboardButton::callback(text, payload))
                        .collect::<Vec<InlineKeyboardButton>>()
                }));
                bot.send_message(user_id, msg).reply_markup(markup).await?;
            }
            CtxResult::Nothing => {}
            CtxResult::NewCtx(_) => unreachable!(),
        }
    }
    Ok(())
}

pub async fn main_state_handler<T: RecordsRepository + 'static>(
    bot: Bot,
    msg: Message,
    context: Arc<RwLock<BotContext<T>>>,
) -> HandlerResult {
    let user_id = msg.from().unwrap().id;
    let mut ctx_results: Vec<CtxResult> = vec![];

    if !context.read().await.dial_ctxs.contains_key(&user_id) {
        ctx_results.extend(initialize_dialog_ctx(context.clone(), user_id).await);
    }
    let rctx = context.read().await;
    let mut dial = rctx.dial_ctxs.get(&user_id).unwrap().write().await;
    if let Some(text) = msg.text() {
        ctx_results.push(dial.handle_input(text).expect("Failed input handling"));
    }

    drop(dial);
    drop(rctx);

    process_ctx_results(user_id, context, bot, ctx_results).await
}

pub async fn default_message_handler<T: RecordsRepository>(
    bot: Bot,
    msg: Message,
    _context: Arc<RwLock<BotContext<T>>>,
) -> HandlerResult {
    log::debug!(
        "Defalt message handler called. chat_id: {}; from: {:?}",
        msg.chat.id,
        msg.from().map(|msg_from| msg_from.id)
    );

    bot.delete_message(msg.chat.id, msg.id).await?;
    Ok(())
}

pub async fn default_callback_handler<T: RecordsRepository + 'static>(
    bot: Bot,
    query: CallbackQuery,
    context: Arc<RwLock<BotContext<T>>>,
) -> HandlerResult {
    log::debug!(
        "Callback: called, chat_id: {:?}; from: {:?}",
        query.chat_id(),
        query.from.id
    );

    let user_id = query.from.id;
    let mut ctx_results: Vec<CtxResult> = vec![];

    if !context.read().await.dial_ctxs.contains_key(&user_id) {
        log::debug!("Callback ({user_id}): dialog context not found");
        ctx_results.extend(initialize_dialog_ctx(context.clone(), user_id).await);
    }

    if let Some(data) = query.data {
        log::debug!("Callback ({user_id}): Handling \"{data}\"");

        let rctx = context.read().await;
        let mut dial = rctx.dial_ctxs.get(&user_id).unwrap().write().await;
        ctx_results.push(dial.handle_select(data.as_str()).unwrap());
    }

    bot.answer_callback_query(query.id).await?;
    if let Some(msg) = query.message {
        bot.delete_message(user_id, msg.id).await?;
    }

    log::debug!("Callback ({user_id}): Processing ctx results");
    process_ctx_results(user_id, context, bot, ctx_results).await
}

pub fn build_handler<T: RecordsRepository + 'static>(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    let messages_hanler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::MainState].endpoint(main_state_handler::<T>))
        .endpoint(default_message_handler::<T>);

    let callbacks_hanlder = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
        .endpoint(default_callback_handler::<T>);

    dptree::entry()
        .branch(messages_hanler)
        .branch(callbacks_hanlder)
}
