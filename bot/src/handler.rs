use std::{collections::HashMap, error::Error, marker::PhantomData};

use sec_store::repository::RecordsRepository;
use teloxide::{
    dispatching::{
        dialogue::{GetChatId, InMemStorage},
        DpHandlerDescription, HandlerExt, UpdateFilterExt,
    },
    dptree,
    payloads::SendMessageSetters,
    prelude::{DependencyMap, Handler},
    requests::Requester,
    types::{
        CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode, Update,
        UserId,
    },
    Bot,
};
use tokio::sync::RwLock;

use crate::{
    stated_dialogues::{hello::HelloDialogue, CtxResult, DialContext, MessageFormat, MessageId},
    user_repo_factory::RepositoriesFactory,
};
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub enum BotState {
    #[default]
    Default,
}

pub struct BotContext<F: RepositoriesFactory<R>, R: RecordsRepository> {
    pub factory: F,
    pub dial_ctxs: HashMap<UserId, RwLock<Box<dyn DialContext + Sync + Send>>>,
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

pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn initialize_dialog_ctx<F: RepositoriesFactory<R>, R: RecordsRepository>(
    context: &RwLock<BotContext<F, R>>,
    user_id: UserId,
) -> Vec<CtxResult> {
    log::debug!("Creating new dialogue with {user_id}...");
    let mut wctx = context.write().await;
    let factory = wctx.factory.clone();
    let mut dial = HelloDialogue::<F, R>::new(user_id.into(), factory);
    let result = dial.init().expect("Failed dialogue initialize");

    wctx.dial_ctxs.insert(user_id, RwLock::new(Box::new(dial)));
    log::debug!("New dialogue with {user_id} created!");

    return result;
}

async fn process_ctx_results<F: RepositoriesFactory<R>, R: RecordsRepository>(
    user_id: UserId,
    context: Arc<RwLock<BotContext<F, R>>>,
    bot: Bot,
    ctx_results: Vec<CtxResult>,
) -> HandlerResult {
    let mut new_ctx_results: Vec<CtxResult> = vec![];
    let mut buckets: Vec<Vec<CtxResult>> = vec![ctx_results];

    loop {
        if buckets.is_empty() {
            if context.read().await.dial_ctxs.contains_key(&user_id) {
                break;
            }
            log::debug!("Results processing ({user_id}): the user was left dialog context");
            buckets.push(initialize_dialog_ctx(&context, user_id).await);
        }

        let mut bucket = buckets.remove(0);
        while !bucket.is_empty() {
            match &bucket[0] {
                CtxResult::NewCtx(_) => break,
                CtxResult::CloseCtx => break,
                _ => new_ctx_results.push(bucket.remove(0)),
            }
        }
        if bucket.is_empty() {
            continue;
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
                    log::debug!("Results processing ({user_id}): calling shutdown for old ctx...");
                    new_bucket.extend(
                        old_ctx
                            .write()
                            .await
                            .shutdown()
                            .expect(format!("Failed ctx shutdown for {}", user_id).as_str()),
                    );
                    log::debug!("Results processing ({user_id}): old ctx finished");
                }
                log::debug!("Results processing ({user_id}): New ctx intialization");
                new_bucket.extend(
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
                log::debug!("Results processing ({user_id}): New ctx initialized");
            }
            CtxResult::CloseCtx => {
                log::debug!("Results processing ({user_id}): close dialog context");
                if let Some(ctx) = context.write().await.dial_ctxs.remove(&user_id) {
                    new_bucket.extend(ctx.write().await.shutdown().unwrap());
                }
            }
            _ => unreachable!(),
        }
        new_bucket.extend(bucket);
        buckets.push(new_bucket);
    }

    log::debug!(
        "Results processing ({user_id}): executing {} results...",
        new_ctx_results.len()
    );
    for ctx_result in new_ctx_results {
        match ctx_result {
            CtxResult::Messages(messages) => {
                log::debug!(
                    "Results processing ({user_id}): sending {} messages",
                    messages.len()
                );
                let mut sent_messages_ids: Vec<MessageId> = Vec::new();
                for msg in messages {
                    bot.send_message(user_id, msg.text())
                        .parse_mode(match msg.format {
                            MessageFormat::Text => ParseMode::MarkdownV2,
                            MessageFormat::Html => ParseMode::Html,
                        })
                        .await
                        .map(|msg| sent_messages_ids.push(msg.id.into()))?;
                }
                context
                    .read()
                    .await
                    .dial_ctxs
                    .get(&user_id)
                    .unwrap()
                    .write()
                    .await
                    .remember_sent_messages(sent_messages_ids);
            }
            CtxResult::Buttons(msg, selector) => {
                log::debug!("Results processing ({user_id}): sending keyboard");
                let markup = InlineKeyboardMarkup::new(selector.into_iter().map(|buttons_row| {
                    buttons_row
                        .into_iter()
                        .map(|(payload, text)| InlineKeyboardButton::callback(text, payload))
                        .collect::<Vec<InlineKeyboardButton>>()
                }));
                let sent_message = bot
                    .send_message(user_id, msg.text())
                    .parse_mode(match msg.format {
                        MessageFormat::Html => ParseMode::Html,
                        MessageFormat::Text => ParseMode::MarkdownV2,
                    })
                    .reply_markup(markup)
                    .await?;
                context
                    .read()
                    .await
                    .dial_ctxs
                    .get(&user_id)
                    .unwrap()
                    .write()
                    .await
                    .remember_sent_messages(vec![sent_message.id.into()]);
            }
            CtxResult::RemoveMessages(messages_ids) => {
                log::debug!(
                    "Results processing ({user_id}): removing {} messages",
                    messages_ids.len()
                );
                for message_id in messages_ids {
                    bot.delete_message(user_id, message_id.into()).await?;
                }
            }
            CtxResult::Nothing => {}
            CtxResult::CloseCtx => unreachable!(),
            CtxResult::NewCtx(_) => unreachable!(),
        }
    }
    log::debug!("Results processing ({user_id}): all results processed");
    Ok(())
}

pub async fn main_state_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
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
    let mut ctx_results: Vec<CtxResult> = vec![];

    if !context.read().await.dial_ctxs.contains_key(&user_id) {
        ctx_results.extend(initialize_dialog_ctx(&context, user_id).await);
    }
    {
        let rctx = context.read().await;
        let mut dial = rctx.dial_ctxs.get(&user_id).unwrap().write().await;
        ctx_results.extend(
            dial.handle_message(msg.into())
                .expect(&format!("({}): Failed input handling", user_id)),
        );
    }

    process_ctx_results(user_id, context, bot, ctx_results).await
}

pub async fn default_callback_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
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
    let mut ctx_results: Vec<CtxResult> = vec![];

    if !context.read().await.dial_ctxs.contains_key(&user_id) {
        log::debug!("Callback ({user_id}): dialog context not found");
        ctx_results.extend(initialize_dialog_ctx(&context, user_id).await);
    }

    log::debug!("Callback ({user_id}): Handling \"{:?}\"", query.data);
    {
        let rctx = context.read().await;
        let mut dial = rctx.dial_ctxs.get(&user_id).unwrap().write().await;
        ctx_results.extend(dial.handle_select(query.into()).unwrap());
    }

    log::debug!("Callback ({user_id}): Processing ctx results");
    process_ctx_results(user_id, context, bot, ctx_results).await
}

pub fn build_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    let messages_hanler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<BotState>, BotState>()
        .endpoint(main_state_handler::<F, R>);

    let callbacks_hanlder = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<BotState>, BotState>()
        .endpoint(default_callback_handler::<F, R>);

    dptree::entry()
        .branch(messages_hanler)
        .branch(callbacks_hanlder)
}
