use std::future::IntoFuture;

use teloxide::{
    payloads::SendMessageSetters,
    requests::Requester,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, UserId},
    Bot,
};
use tokio::{sync::RwLock, task::JoinSet};

use crate::dialogues_controller::DialCtxActions;
use crate::{
    dialogues_controller::{self, DialInteraction},
    stated_dialogues::{MessageFormat, MessageId},
};

use super::handlers::{AnyResult, HandlerResult};

pub async fn handle_interaction<T: DialCtxActions>(
    user_id: &UserId,
    bot: &Bot,
    context: &RwLock<T>,
    interaction: DialInteraction,
) -> HandlerResult {
    let dial_controller = context.write().await.take_controller(&user_id.0);

    let (controller, results) = match dial_controller {
        Some(controller) => controller.handle(interaction),
        None => {
            let (controller, results) = context.read().await.new_controller(user_id.0)?;
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
        context.write().await.put_controller(user_id.0, controller);
    } else {
        let (mut controller, results) = context.read().await.new_controller(user_id.0)?;
        let sent_msg_ids = process_ctx_results(*user_id, results, bot).await?;
        controller.remember_sent_messages(sent_msg_ids);
        context.write().await.put_controller(user_id.0, controller);
    }
    Ok(())
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
