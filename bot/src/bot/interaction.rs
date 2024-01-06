use std::future::IntoFuture;

use sec_store::repository::RecordsRepository;
use teloxide::{
    payloads::SendMessageSetters,
    requests::Requester,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, UserId},
    Bot,
};
use tokio::{sync::RwLock, task::JoinSet};

use crate::{dialogues::hello::HelloDialogue, user_repo_factory::RepositoriesFactory};
use crate::{
    dialogues_controller::{self, DialInteraction, DialogueController},
    stated_dialogues::{MessageFormat, MessageId},
};
use std::sync::Arc;

use super::{
    handlers::{AnyResult, HandlerResult},
    BotContext,
};

pub async fn handle_interaction<F: RepositoriesFactory<R>, R: RecordsRepository>(
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

pub async fn process_ctx_results(
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
