use std::error::Error;

use sec_store::repository::RecordsRepository;
use teloxide::{
    dispatching::{
        dialogue::{GetChatId, InMemStorage},
        DpHandlerDescription, HandlerExt, UpdateFilterExt,
    },
    dptree, filter_command,
    prelude::{DependencyMap, Handler},
    types::{CallbackQuery, Message, Update},
    Bot,
};
use tokio::sync::RwLock;

use crate::bot::interaction::{handle_interaction, process_ctx_results};
use crate::dialogues_controller::DialInteraction;
use crate::user_repo_factory::RepositoriesFactory;
use std::sync::Arc;

use super::{BotContext, BotState, Command};

pub type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
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
