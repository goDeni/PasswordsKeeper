#![allow(dead_code)]

use stated_dialogues::Select;

pub mod dialogues;
pub mod dialogues_controller;
pub mod handler;
pub mod stated_dialogues;
pub mod user_repo_factory;

impl From<teloxide::types::MessageId> for stated_dialogues::MessageId {
    fn from(val: teloxide::types::MessageId) -> Self {
        stated_dialogues::MessageId(val.0)
    }
}

impl From<stated_dialogues::MessageId> for teloxide::types::MessageId {
    fn from(val: stated_dialogues::MessageId) -> Self {
        teloxide::types::MessageId(val.0)
    }
}

impl From<teloxide::types::UserId> for stated_dialogues::UserId {
    fn from(val: teloxide::types::UserId) -> Self {
        stated_dialogues::UserId(val.0.to_string())
    }
}

impl From<teloxide::types::Message> for stated_dialogues::Message {
    fn from(val: teloxide::types::Message) -> Self {
        stated_dialogues::Message::new(
            val.id.into(),
            val.text().map(|t| t.to_string()),
            val.from().map(|user| user.id.into()),
        )
    }
}

impl From<teloxide::types::CallbackQuery> for stated_dialogues::Select {
    fn from(val: teloxide::types::CallbackQuery) -> Self {
        Select::new(
            val.message.map(|msg| msg.id.into()),
            val.data,
            val.from.id.into(),
        )
    }
}
