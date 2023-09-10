#![allow(dead_code)]

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

impl Into<stated_dialogues::Message> for teloxide::types::Message {
    fn into(self) -> stated_dialogues::Message {
        stated_dialogues::Message(self.id.into(), self.text().map(|text| text.to_string()))
    }
}
