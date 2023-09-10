mod create_repo;
pub mod hello;

use std::fmt::Debug;

use anyhow::Result;

enum DialogState {
    WaitForInput,
    WaitForSelect,
    IDLE,
}

pub enum CtxResult {
    Messages(Vec<String>),
    RemoveMessages(Vec<MessageId>),
    Buttons(String, Vec<Vec<(ButtonPayload, String)>>),
    NewCtx(Box<dyn DialContext + Send + Sync + 'static>),
    Nothing,
}

impl Debug for CtxResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Messages(arg0) => f.debug_tuple("Messages").field(arg0).finish(),
            Self::Buttons(arg0, arg1) => f.debug_tuple("Buttons").field(arg0).field(arg1).finish(),
            Self::NewCtx(_) => f.debug_tuple("NewCtx(?)").finish(),
            Self::Nothing => write!(f, "Nothing"),
            Self::RemoveMessages(arg0) => f.debug_tuple("RemoveMessages").field(arg0).finish(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DialogueId(pub String);

#[derive(Clone, Debug)]
pub struct MessageId(pub i32);

#[derive(Clone)]
pub struct Message(pub MessageId, pub Option<String>);
impl Message {
    fn id(&self) -> &MessageId {
        &self.0
    }
    fn text(&self) -> Option<&str> {
        match &self.1 {
            Some(text) => Some(text),
            None => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ButtonPayload(String);
impl Into<String> for ButtonPayload {
    fn into(self) -> String {
        self.0
    }
}

pub trait DialContext {
    //
    fn init(&mut self) -> Result<CtxResult>;
    fn shutdown(&self) -> Result<CtxResult>;
    //
    fn handle_select(&mut self, select: &str) -> Result<CtxResult>;
    fn handle_message(&mut self, input: Message) -> Result<CtxResult>;
    fn handle_command(&mut self, command: &str) -> Result<CtxResult>;
    //
}
