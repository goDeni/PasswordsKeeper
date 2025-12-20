use sec_store::repository::RecordsRepository;
use teloxide::{
    dispatching::{
        dialogue::{GetChatId, InMemStorage},
        DpHandlerDescription, HandlerExt, UpdateFilterExt,
    },
    dptree, filter_command,
    net::Download,
    prelude::Handler,
    requests::Requester,
    types::{CallbackQuery, Message, Update},
    Bot,
};

use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Context;
use stated_dialogues::controller::handler::{dialog_expect_file, handle_interaction};
use stated_dialogues::controller::{teloxide::HandlerResult, DialInteraction};
use std::fs::exists as file_exists;
use std::sync::Arc;
use tempfile::Builder as TmpFileBuilder;
use tempfile::TempDir;
use tokio::fs::File as TokioFile;

use super::{BotContext, BotState, Command};

pub fn build_handler<F: RepositoriesFactory<R>, R: RecordsRepository>() -> Handler<
    'static,
    std::result::Result<(), std::boxed::Box<dyn std::error::Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    let commands_handler = filter_command::<Command, _>().endpoint(handle_command::<F, R>);

    let messages_hanler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<BotState>, BotState>()
        .branch(commands_handler)
        .endpoint(main_state_handler::<F, R>);

    let callbacks_hanlder = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<BotState>, BotState>()
        .endpoint(default_callback_handler::<F, R>);

    dptree::entry()
        .branch(dptree::filter(filter_updates::<F, R>).endpoint(non_allowed_updates_handler))
        .branch(messages_hanler)
        .branch(callbacks_hanlder)
}

fn filter_updates<F: RepositoriesFactory<R>, R: RecordsRepository>(
    update: Update,
    context: Arc<BotContext<F, R>>,
) -> bool {
    log::debug!(
        "Filter update. id={:?} from={:?}",
        update.id,
        update.from().map(|f| f.id)
    );

    if let Some(user) = update.from() {
        if context.whitelist.check_allowed(&user.id) {
            return false;
        }
    }
    true
}

async fn non_allowed_updates_handler(bot: Bot, update: Update) -> HandlerResult {
    log::debug!(
        "Handle skipped update. id={:?} from={:?} chat_id={:?}",
        update.id,
        update.from().map(|f| f.id),
        update.chat_id(),
    );

    if let Some(chat_id) = update
        .from()
        .map_or_else(|| update.chat_id(), |user| Some(user.id.into()))
    {
        bot.send_message(
            chat_id,
            format!(
                "Вам не разрешено пользоваться этим ботом. Ваш идентификатор \"{}\"",
                chat_id
            ),
        )
        .await?;
    }

    Ok(())
}
async fn main_state_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
    bot: Bot,
    msg: Message,
    context: Arc<BotContext<F, R>>,
    docments_tempdir: Arc<TempDir>,
) -> HandlerResult {
    log::debug!(
        "Handling message. chat_id={} from={:?}",
        msg.chat.id,
        msg.from.clone().map(|f| f.id)
    );

    let user_id = msg.from.clone().unwrap().id;
    match (
        msg.document(),
        dialog_expect_file(&user_id.0, &context.dial).await,
    ) {
        (Some(document), true) => {
            log::debug!(
                "Got document id={} size={}. chat_id={}",
                document.file.id,
                document.file.size,
                msg.chat.id,
            );

            let file = bot.get_file(document.file.id.clone()).await?;
            let tmpfile = TmpFileBuilder::new()
                .prefix(&format!("document_{}_", file.id))
                .tempfile_in(docments_tempdir.as_ref())
                .with_context(|| "Failed temporary file creation".to_string())?;

            let mut file_fd = TokioFile::options()
                .write(true)
                .open(tmpfile.path())
                .await?;
            bot.download_file(&file.path, &mut file_fd).await?;
            file_fd.sync_all().await?;

            log::debug!(
                "File downloaded. filename={:?} size={} chat_id={}",
                tmpfile.path().file_name(),
                document.file.size,
                msg.chat.id,
            );

            let res = handle_interaction(
                &user_id.0,
                &context.bot_adapter,
                &context.dial,
                DialInteraction::Message((msg, tmpfile.path().to_path_buf()).into()),
            )
            .await;
            if file_exists(&tmpfile)? {
                tmpfile.close()?;
            }

            res
        }
        _ => {
            handle_interaction(
                &user_id.0,
                &context.bot_adapter,
                &context.dial,
                DialInteraction::Message(msg.into()),
            )
            .await
        }
    }
}

async fn default_callback_handler<F: RepositoriesFactory<R>, R: RecordsRepository>(
    query: CallbackQuery,
    context: Arc<BotContext<F, R>>,
) -> HandlerResult {
    log::debug!("Callback: called, chat_id: {:?}", query.chat_id(),);

    let user_id = query.from.id;
    log::debug!("Callback: Handling \"{:?}\"", query.data);
    handle_interaction(
        &user_id.0,
        &context.bot_adapter,
        &context.dial,
        DialInteraction::Select(query.into()),
    )
    .await
}

async fn handle_command<F: RepositoriesFactory<R>, R: RecordsRepository>(
    msg: Message,
    context: Arc<BotContext<F, R>>,
) -> HandlerResult {
    log::debug!("Handling {:?} command. chat_id={}", msg.text(), msg.chat.id,);
    let user_id = msg.from.clone().unwrap().id;
    handle_interaction(
        &user_id.0,
        &context.bot_adapter,
        &context.dial,
        DialInteraction::Command(msg.clone().into()),
    )
    .await
}
