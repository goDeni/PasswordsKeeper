pub mod handler;
pub mod teloxide;
pub mod ttl;

use std::{future::Future, time::SystemTime};

use crate::stated_dialogues::{self, ButtonPayload, DialContext, MessageId, OutgoingMessage};
use anyhow::{Context, Result};

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
    Select(stated_dialogues::Select),
    Message(stated_dialogues::Message),
    Command(stated_dialogues::Message),
}

pub enum CtxResult {
    Messages(Vec<OutgoingMessage>),
    RemoveMessages(Vec<MessageId>),
    Buttons(OutgoingMessage, Vec<Vec<(ButtonPayload, String)>>),
}

pub trait DialCtxActions {
    fn new_controller(&self, user_id: u64) -> Result<(DialogueController, Vec<CtxResult>)>;
    fn take_controller(&mut self, user_id: &u64) -> Option<DialogueController>;
    fn put_controller(&mut self, user_id: u64, controller: DialogueController);
    fn dialogues_list(&self) -> Vec<(&u64, &DialogueController)>;
}

impl DialogueController {
    pub fn create<T>(mut context: T) -> Result<(Self, Vec<CtxResult>)>
    where
        T: DialContext + Sync + Send + 'static,
    {
        let results = context.init()?;
        let (context, results) = process_context_results(Box::new(context), results)?;
        Ok((
            DialogueController {
                context: context
                    .with_context(|| "context self destroyed after initialization".to_string())?,
                last_usage_time: SystemTime::now(),
            },
            results,
        ))
    }

    pub fn get_last_interaction_time(&self) -> &SystemTime {
        &self.last_usage_time
    }

    pub fn handle(
        mut self,
        interaction: DialInteraction,
    ) -> Result<(Option<Self>, Vec<CtxResult>)> {
        let results = match interaction {
            DialInteraction::Select(select) => self.context.handle_select(select),
            DialInteraction::Message(message) => self.context.handle_message(message),
            DialInteraction::Command(command) => self.context.handle_command(command),
        }?;

        let (context, results) = process_context_results(self.context, results)?;

        Ok((
            context.map(|ctx| DialogueController {
                context: ctx,
                last_usage_time: SystemTime::now(),
            }),
            results,
        ))
    }

    pub fn shutdown(mut self) -> Result<Vec<CtxResult>> {
        let results = self.context.shutdown()?;
        process_context_results(self.context, results).map(|(_, ctx_results)| ctx_results)
    }

    pub fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        self.context.remember_sent_messages(msg_ids)
    }
}

fn process_context_results(
    context: Box<AnyDialContext>,
    mut results: Vec<stated_dialogues::CtxResult>,
) -> Result<(Option<Box<AnyDialContext>>, Vec<CtxResult>)> {
    let mut context: Option<Box<AnyDialContext>> = Some(context);

    loop {
        let mut new_results: Vec<stated_dialogues::CtxResult> = vec![];
        for ctx_result in results {
            match ctx_result {
                stated_dialogues::CtxResult::NewCtx(mut new_ctx) => {
                    if let Some(ref mut old_ctx) = context {
                        new_results.extend(old_ctx.shutdown()?);
                    }
                    new_results.extend(new_ctx.init()?);
                    context = Some(new_ctx);
                }
                stated_dialogues::CtxResult::CloseCtx => {
                    if let Some(ref mut old_ctx) = context {
                        new_results.extend(old_ctx.shutdown()?);
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
                stated_dialogues::CtxResult::CloseCtx | stated_dialogues::CtxResult::NewCtx(_)
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
                stated_dialogues::CtxResult::Messages(messages) => {
                    Some(CtxResult::Messages(messages))
                }
                stated_dialogues::CtxResult::RemoveMessages(msg_ids) => {
                    Some(CtxResult::RemoveMessages(msg_ids))
                }
                stated_dialogues::CtxResult::Buttons(msg, buttons) => {
                    Some(CtxResult::Buttons(msg, buttons))
                }
                stated_dialogues::CtxResult::Nothing => None,
                stated_dialogues::CtxResult::CloseCtx => unreachable!(),
                stated_dialogues::CtxResult::NewCtx(_) => unreachable!(),
            })
            .collect::<Vec<CtxResult>>(),
    ))
}
