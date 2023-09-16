#![allow(dead_code)]

use stated_dialogues::Select;

pub mod handler;
pub mod reps_store;
pub mod stated_dialogues;
pub mod user_repo_factory;

impl Into<stated_dialogues::MessageId> for teloxide::types::MessageId {
    fn into(self) -> stated_dialogues::MessageId {
        stated_dialogues::MessageId(self.0)
    }
}

impl Into<teloxide::types::MessageId> for stated_dialogues::MessageId {
    fn into(self) -> teloxide::types::MessageId {
        teloxide::types::MessageId(self.0)
    }
}

impl Into<stated_dialogues::UserId> for teloxide::types::UserId {
    fn into(self) -> stated_dialogues::UserId {
        stated_dialogues::UserId(self.0.to_string())
    }
}

impl Into<stated_dialogues::Message> for teloxide::types::Message {
    fn into(self) -> stated_dialogues::Message {
        stated_dialogues::Message::new(
            self.id.into(),
            self.text().map(|t| t.to_string()),
            self.from().map(|user| user.id.into()),
        )
    }
}

impl Into<stated_dialogues::Select> for teloxide::types::CallbackQuery {
    fn into(self) -> stated_dialogues::Select {
        Select::new(
            self.message.map(|msg| msg.id.into()),
            self.data,
            self.from.id.into(),
        )
    }
}
