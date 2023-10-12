use anyhow::Result;
use std::fmt::{Debug, Display};

#[derive(Debug, PartialEq)]
pub enum MessageFormat {
    Text,
    Html,
}

#[derive(Debug, PartialEq)]
pub struct OutgoingMessage {
    text: String,
    pub format: MessageFormat,
}
impl OutgoingMessage {
    pub fn new(text: String, format: MessageFormat) -> Self {
        OutgoingMessage { text, format }
    }

    pub fn text(&self) -> &str {
        return &self.text;
    }
}

impl Into<OutgoingMessage> for String {
    fn into(self) -> OutgoingMessage {
        OutgoingMessage::new(self, MessageFormat::Text)
    }
}

impl Into<OutgoingMessage> for &str {
    fn into(self) -> OutgoingMessage {
        OutgoingMessage::new(self.into(), MessageFormat::Text)
    }
}

impl Into<String> for OutgoingMessage {
    fn into(self) -> String {
        self.text
    }
}

pub enum CtxResult {
    Messages(Vec<OutgoingMessage>),
    RemoveMessages(Vec<MessageId>),
    Buttons(OutgoingMessage, Vec<Vec<(ButtonPayload, String)>>),
    NewCtx(Box<dyn DialContext + Send + Sync + 'static>),
    CloseCtx,
    Nothing,
}

impl Debug for CtxResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Messages(arg0) => f.debug_tuple("Messages").field(arg0).finish(),
            Self::Buttons(arg0, arg1) => f.debug_tuple("Buttons").field(arg0).field(arg1).finish(),
            Self::NewCtx(_) => f.debug_tuple("NewCtx(?)").finish(),
            Self::Nothing => write!(f, "Nothing"),
            Self::CloseCtx => write!(f, "CloseCtx"),
            Self::RemoveMessages(arg0) => f.debug_tuple("RemoveMessages").field(arg0).finish(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MessageId(pub i32);
#[derive(Clone, Debug)]
pub struct UserId(pub String);
impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Into<String> for UserId {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Clone)]
pub struct Message {
    pub id: MessageId,
    pub text: Option<String>,
    pub user_id: Option<UserId>,
}

impl Message {
    pub fn new(id: MessageId, text: Option<String>, user_id: Option<UserId>) -> Self {
        Message { id, text, user_id }
    }
    pub fn text(&self) -> Option<&str> {
        match &self.text {
            Some(text) => Some(text),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Select {
    pub msg_id: Option<MessageId>,
    pub data: Option<String>,
    pub user_id: UserId,
}
impl Select {
    pub fn new(msg_id: Option<MessageId>, data: Option<String>, user_id: UserId) -> Self {
        Select {
            msg_id,
            data,
            user_id,
        }
    }

    pub fn data(&self) -> Option<&str> {
        match &self.data {
            Some(data) => Some(data),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ButtonPayload(pub String);
impl Into<String> for ButtonPayload {
    fn into(self) -> String {
        self.0
    }
}
impl Into<ButtonPayload> for String {
    fn into(self) -> ButtonPayload {
        ButtonPayload(self)
    }
}
impl Into<ButtonPayload> for &str {
    fn into(self) -> ButtonPayload {
        self.to_string().into()
    }
}

pub trait DialContext {
    //
    fn init(&mut self) -> Result<Vec<CtxResult>>;
    fn shutdown(&mut self) -> Result<Vec<CtxResult>>;
    //
    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>>;
    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>>;
    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>>;
    //
    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>);
}
