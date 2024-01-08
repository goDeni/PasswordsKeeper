use std::collections::HashSet;
use std::marker::PhantomData;

use crate::{dialogues::commands::CANCEL_COMMAND, user_repo_factory::RepositoriesFactory};

use super::view_repo::ViewRepoDialog;
use anyhow::{Context, Result};
use sec_store::repository::{RecordsRepository, RepositoryOpenError};
use stated_dialogues::stated_dialogues::{CtxResult, DialContext, Message, MessageId, Select};

pub struct OpenRepoDialogue<F, R> {
    factory: F,
    sent_msg_ids: HashSet<MessageId>,
    //
    phantom: PhantomData<R>,
}

impl<F, R> OpenRepoDialogue<F, R> {
    pub fn new(factory: F) -> Self {
        OpenRepoDialogue {
            factory,
            sent_msg_ids: HashSet::new(),
            phantom: PhantomData,
        }
    }
}

impl<F, R> DialContext for OpenRepoDialogue<F, R>
where
    R: RecordsRepository,
    F: RepositoriesFactory<R>,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::Messages(vec!["Введите пароль".into()])])
    }

    fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        Ok(vec![select
            .msg_id
            .map(|msg_id| CtxResult::RemoveMessages(vec![msg_id]))
            .unwrap_or(CtxResult::Nothing)])
    }

    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![
            CtxResult::RemoveMessages(
                self.sent_msg_ids
                    .drain()
                    .chain(vec![message.id.clone()])
                    .collect(),
            ),
            match (
                message
                    .user_id
                    .with_context(|| format!("Message without user_id msg_id={}", message.id,))?,
                message.text,
            ) {
                (user_id, Some(passwd)) => {
                    match self
                        .factory
                        .get_user_repository(&user_id.clone().into(), passwd)
                    {
                        Ok(repo) => Ok(CtxResult::NewCtx(Box::new(ViewRepoDialog::new(repo)))),
                        Err(RepositoryOpenError::WrongPassword) => Ok(CtxResult::Messages(vec![
                            "Пароль не подходит 🤨. Попробуйте еще раз".into(),
                        ])),
                        Err(RepositoryOpenError::DoesntExist) => Ok(CtxResult::CloseCtx),
                        Err(error) => Err(error),
                    }
                }
                _ => Ok(CtxResult::Messages(vec![
                    "Это не пароль 🤨. Попробуйте еще раз".into(),
                ])),
            }?,
        ])
    }

    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        match command.text() {
            Some(CANCEL_COMMAND) => Ok(vec![
                CtxResult::RemoveMessages(vec![command.id]),
                CtxResult::CloseCtx,
            ]),
            _ => Ok(vec![CtxResult::RemoveMessages(vec![command.id])]),
        }
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
}
