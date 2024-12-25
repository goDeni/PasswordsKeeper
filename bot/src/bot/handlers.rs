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

use crate::user_repo_factory::RepositoriesFactory;
use stated_dialogues::controller::{
    handler::{handle_interaction, process_ctx_results},
    DialCtxActions,
};
use stated_dialogues::controller::{teloxide::HandlerResult, DialInteraction};
use std::sync::Arc;

use super::{BotContext, BotState, Command};

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
    msg: Message,
    context: Arc<BotContext<F, R>>,
) -> HandlerResult {
    log::debug!(
        "Handling message. chat_id={} from={:?}",
        msg.chat.id,
        msg.from.clone().map(|f| f.id)
    );

    let user_id = msg.from.clone().unwrap().id;
    handle_interaction(
        &user_id.0,
        &context.bot_adapter,
        &context.dial,
        DialInteraction::Message(msg.into()),
    )
    .await
}

async fn default_callback_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
    _bot: Bot,
    query: CallbackQuery,
    context: Arc<BotContext<F, R>>,
) -> HandlerResult {
    log::debug!(
        "Callback: called, chat_id: {:?}; from: {:?}",
        query.chat_id(),
        query.from.id
    );

    let user_id = query.from.id;
    log::debug!("Callback ({user_id}): Handling \"{:?}\"", query.data);
    handle_interaction(
        &user_id.0,
        &context.bot_adapter,
        &context.dial,
        DialInteraction::Select(query.into()),
    )
    .await
}

async fn handle_reset_command<F: RepositoriesFactory<R>, R: RecordsRepository>(
    msg: Message,
    context: Arc<BotContext<F, R>>,
) -> HandlerResult {
    log::debug!(
        "Handling reset command. chat_id={} from={:?}",
        msg.chat.id,
        msg.from.clone().map(|f| f.id)
    );
    let user_id = msg.from.clone().unwrap().id;
    if let Some(old_controller) = context.dial.write().await.take_controller(&user_id.0) {
        process_ctx_results(
            user_id.0,
            old_controller.shutdown().await?,
            &context.bot_adapter,
        )
        .await?;
    }

    handle_interaction(
        &user_id.0,
        &context.bot_adapter,
        &context.dial,
        DialInteraction::Command(msg.clone().into()),
    )
    .await
}

async fn handle_command<F: RepositoriesFactory<R>, R: RecordsRepository>(
    msg: Message,
    context: Arc<BotContext<F, R>>,
) -> HandlerResult {
    log::debug!(
        "Handling {:?} command. chat_id={} from={:?}",
        msg.text(),
        msg.chat.id,
        msg.from.clone().map(|f| f.id)
    );
    let user_id = msg.from.clone().unwrap().id;
    handle_interaction(
        &user_id.0,
        &context.bot_adapter,
        &context.dial,
        DialInteraction::Command(msg.clone().into()),
    )
    .await
}
