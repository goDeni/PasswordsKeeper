use std::future::IntoFuture;

use anyhow::Context;
use teloxide::payloads::SendMessageSetters;
use teloxide::requests::Requester;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, UserId};
use teloxide::Bot;
use tokio::task::JoinSet;

use super::BotAdapter;
use crate::stated_dialogues::{ButtonPayload, MessageFormat, MessageId, OutgoingMessage};
use anyhow::Result;

pub type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type HandlerResult = AnyResult<()>;

pub struct TeloxideAdapter {
    bot: Bot,
}

impl TeloxideAdapter {
    pub fn new(bot: Bot) -> Self {
        TeloxideAdapter { bot }
    }
}

impl BotAdapter for TeloxideAdapter {
    async fn send_message(&self, user_id: u64, msg: OutgoingMessage) -> Result<MessageId> {
        let send_request = self.bot.send_message(UserId(user_id), msg.text());
        let send_request = match msg.format {
            MessageFormat::Html => send_request.parse_mode(ParseMode::Html),
            MessageFormat::Text => send_request,
        };

        send_request
            .await
            .map(|msg| msg.id.into())
            .with_context(|| format!("Failed message for {user_id} sending"))
    }

    async fn send_keyboard(
        &self,
        user_id: u64,
        msg: OutgoingMessage,
        selector: Vec<Vec<(ButtonPayload, String)>>,
    ) -> Result<MessageId> {
        let markup = InlineKeyboardMarkup::new(selector.into_iter().map(|buttons_row| {
            buttons_row
                .into_iter()
                .map(|(payload, text)| InlineKeyboardButton::callback(text, payload))
                .collect::<Vec<InlineKeyboardButton>>()
        }));
        let send_message = self.bot.send_message(UserId(user_id), msg.text());
        let send_message = match msg.format {
            MessageFormat::Html => send_message.parse_mode(ParseMode::Html),
            MessageFormat::Text => send_message,
        };

        send_message
            .reply_markup(markup)
            .await
            .map(|msg| msg.id.into())
            .with_context(|| format!("Failed keyboard for {user_id} sending"))
    }

    async fn delete_messages(&self, user_id: u64, messages_ids: Vec<MessageId>) -> Result<()> {
        let mut set = JoinSet::new();
        messages_ids
            .into_iter()
            .map(|msg_id| {
                self.bot
                    .delete_message(UserId(user_id), msg_id.into())
                    .into_future()
            })
            .for_each(|future| {
                set.spawn(future);
            });

        while let Some(res) = set.join_next().await {
            if let Err(err) = res? {
                log::error!("Failed message deletion: {}", err);
            }
        }

        Ok(())
    }
}
