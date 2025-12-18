pub mod handler;
#[cfg(any(
    feature = "teloxide-adapter-rustls",
    feature = "teloxide-adapter-native-tls"
))]
pub mod teloxide;
pub mod ttl;

use std::{future::Future, time::SystemTime};

use crate::dialogues::{
    self, ButtonPayload, DialContext, MessageId, OutgoingDocument, OutgoingMessage,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{instrument, Level};

type AnyDialContext = dyn DialContext + Sync + Send;
pub struct DialogueController {
    context: Box<AnyDialContext>,
    last_usage_time: SystemTime,
}

pub trait BotAdapter {
    fn send_message(
        &self,
        user_id: u64,
        msg: OutgoingMessage,
    ) -> impl Future<Output = Result<MessageId>> + Send;
    fn send_document(
        &self,
        user_id: u64,
        msg: OutgoingDocument,
    ) -> impl Future<Output = Result<MessageId>> + Send;
    fn send_keyboard(
        &self,
        user_id: u64,
        msg: OutgoingMessage,
        selector: Vec<Vec<(ButtonPayload, String)>>,
    ) -> impl std::future::Future<Output = Result<MessageId>> + Send;
    //
    fn delete_messages(
        &self,
        user_id: u64,
        messages_ids: Vec<MessageId>,
    ) -> impl Future<Output = Result<()>> + Send;
    fn delete_message(
        &self,
        user_id: u64,
        msg_id: MessageId,
    ) -> impl Future<Output = Result<()>> + Send {
        self.delete_messages(user_id, vec![msg_id])
    }
}

pub enum DialInteraction {
    Select(dialogues::Select),
    Message(dialogues::Message),
    Command(dialogues::Message),
}

pub enum CtxResult {
    Messages(Vec<OutgoingMessage>),
    Document(OutgoingDocument),
    RemoveMessages(Vec<MessageId>),
    Buttons(OutgoingMessage, Vec<Vec<(ButtonPayload, String)>>),
}

#[async_trait]
pub trait DialCtxActions {
    async fn new_controller(&self, user_id: u64) -> Result<(DialogueController, Vec<CtxResult>)>;
    fn get_controller(&self, user_id: &u64) -> Option<&DialogueController>;
    fn take_controller(&mut self, user_id: &u64) -> Option<DialogueController>;
    fn put_controller(&mut self, user_id: u64, controller: DialogueController);
    fn dialogues_list(&self) -> Vec<(&u64, &DialogueController)>;
}

impl DialogueController {
    pub async fn create<T>(mut context: T) -> Result<(Self, Vec<CtxResult>)>
    where
        T: DialContext + Sync + Send + 'static,
    {
        let results = context.init().await?;
        let (context, results) = process_context_results(Box::new(context), results).await?;
        Ok((
            DialogueController {
                context: context.context("context self destroyed after initialization")?,
                last_usage_time: SystemTime::now(),
            },
            results,
        ))
    }

    pub fn get_last_interaction_time(&self) -> &SystemTime {
        &self.last_usage_time
    }

    pub fn file_expected(&self) -> bool {
        self.context.file_expected()
    }

    pub async fn handle(
        mut self,
        interaction: DialInteraction,
    ) -> Result<(Option<Self>, Vec<CtxResult>)> {
        let results = match interaction {
            DialInteraction::Select(select) => self.context.handle_select(select).await,
            DialInteraction::Message(message) => self.context.handle_message(message).await,
            DialInteraction::Command(command) => self.context.handle_command(command).await,
        }?;

        let (context, results) = process_context_results(self.context, results).await?;

        Ok((
            context.map(|ctx| DialogueController {
                context: ctx,
                last_usage_time: SystemTime::now(),
            }),
            results,
        ))
    }

    pub async fn shutdown(mut self) -> Result<Vec<CtxResult>> {
        let results = self.context.shutdown().await?;
        process_context_results(self.context, results)
            .await
            .map(|(_, ctx_results)| ctx_results)
    }

    pub fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        self.context.remember_sent_messages(msg_ids)
    }
}

#[instrument(level = Level::DEBUG, skip_all)]
async fn process_context_results(
    context: Box<AnyDialContext>,
    mut results: Vec<dialogues::CtxResult>,
) -> Result<(Option<Box<AnyDialContext>>, Vec<CtxResult>)> {
    let mut context: Option<Box<AnyDialContext>> = Some(context);

    loop {
        let mut new_results: Vec<dialogues::CtxResult> = vec![];
        for ctx_result in results {
            match ctx_result {
                dialogues::CtxResult::NewCtx(mut new_ctx) => {
                    if let Some(ref mut old_ctx) = context {
                        new_results.extend(old_ctx.shutdown().await?);
                    }
                    new_results.extend(new_ctx.init().await?);
                    context = Some(new_ctx);
                }
                dialogues::CtxResult::CloseCtx => {
                    if let Some(ref mut old_ctx) = context {
                        new_results.extend(old_ctx.shutdown().await?);
                    }
                    context = None
                }
                others => new_results.push(others),
            };
        }

        results = new_results;
        if !results.iter().any(|res| {
            matches!(
                res,
                dialogues::CtxResult::CloseCtx | dialogues::CtxResult::NewCtx(_)
            )
        }) {
            break;
        }
    }

    Ok((
        context,
        results
            .into_iter()
            .filter_map(|result| match result {
                dialogues::CtxResult::Messages(messages) => Some(CtxResult::Messages(messages)),
                dialogues::CtxResult::Document(documents) => Some(CtxResult::Document(documents)),
                dialogues::CtxResult::RemoveMessages(msg_ids) => {
                    Some(CtxResult::RemoveMessages(msg_ids))
                }
                dialogues::CtxResult::Buttons(msg, buttons) => {
                    Some(CtxResult::Buttons(msg, buttons))
                }
                dialogues::CtxResult::Nothing => None,
                dialogues::CtxResult::CloseCtx => unreachable!(),
                dialogues::CtxResult::NewCtx(_) => unreachable!(),
            })
            .collect::<Vec<CtxResult>>(),
    ))
}
