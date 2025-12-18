use std::future::IntoFuture;

use anyhow::Context;
use teloxide::payloads::SendMessageSetters;
use teloxide::requests::Requester;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile, ParseMode, UserId};
use teloxide::Bot;
use tokio::task::JoinSet;
use tracing::{instrument, Level};

use super::BotAdapter;
use crate::dialogues::{
    self, ButtonPayload, MessageFormat, MessageId, OutgoingDocument, OutgoingMessage,
};
use anyhow::Result;

pub type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type HandlerResult = AnyResult<()>;

impl From<OutgoingDocument> for teloxide::types::InputFile {
    fn from(val: OutgoingDocument) -> Self {
        InputFile::memory(val.data).file_name(val.name)
    }
}

impl From<teloxide::types::MessageId> for dialogues::MessageId {
    fn from(val: teloxide::types::MessageId) -> Self {
        dialogues::MessageId(val.0)
    }
}

impl From<dialogues::MessageId> for teloxide::types::MessageId {
    fn from(val: dialogues::MessageId) -> Self {
        teloxide::types::MessageId(val.0)
    }
}

impl From<teloxide::types::UserId> for dialogues::UserId {
    fn from(val: teloxide::types::UserId) -> Self {
        dialogues::UserId(val.0.to_string())
    }
}

impl From<teloxide::types::Message> for dialogues::Message {
    fn from(val: teloxide::types::Message) -> Self {
        dialogues::Message::new(
            val.id.into(),
            val.text().map(|t| t.to_string()),
            val.from.map(|user| user.id.into()),
            None,
        )
    }
}
impl From<(teloxide::types::Message, std::path::PathBuf)> for dialogues::Message {
    fn from(val: (teloxide::types::Message, std::path::PathBuf)) -> Self {
        let (msg, path) = val;
        dialogues::Message::new(
            msg.id.into(),
            msg.text().map(|t| t.to_string()),
            msg.from.map(|user| user.id.into()),
            Some(path),
        )
    }
}

impl From<teloxide::types::CallbackQuery> for dialogues::Select {
    fn from(val: teloxide::types::CallbackQuery) -> Self {
        dialogues::Select::new(
            val.message.map(|msg| msg.id().into()),
            val.data,
            val.from.id.into(),
        )
    }
}

#[derive(Clone)]
pub struct TeloxideAdapter {
    bot: Bot,
}

impl TeloxideAdapter {
    pub fn new(bot: Bot) -> Self {
        TeloxideAdapter { bot }
    }
}

impl BotAdapter for TeloxideAdapter {
    #[instrument(level = Level::DEBUG, skip(self, msg))]
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

    #[instrument(level = Level::DEBUG, skip(self, document))]
    async fn send_document(&self, user_id: u64, document: OutgoingDocument) -> Result<MessageId> {
        let send_request = self.bot.send_document(UserId(user_id), document.into());
        send_request
            .await
            .map(|msg| msg.id.into())
            .with_context(|| format!("Failed message for {user_id} sending"))
    }

    #[instrument(level = Level::DEBUG, skip(self, msg, selector))]
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

    #[instrument(level = Level::DEBUG, skip(self))]
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
                tracing::error!("Failed message deletion: {}", err);
            }
        }

        Ok(())
    }
}
