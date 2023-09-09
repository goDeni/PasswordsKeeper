mod create_repo;
pub mod hello;

use std::fmt::Debug;

use anyhow::Result;

enum State {
    WaitForInput,
    WaitForSelect,
    IDLE,
}

pub enum CtxResult {
    Messages(Vec<String>),
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
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DialogueId(pub String);

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
    fn handle_input(&mut self, input: &str) -> Result<CtxResult>;
    fn handle_command<C>(&mut self, command: C) -> Result<CtxResult>
    where
        C: AsRef<str>,
        Self: Sized;
    //
}
