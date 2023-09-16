use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::user_repo_factory::RepositoriesFactory;

use super::hello::HelloDialogue;
use super::view_repo::ViewRepoDialog;
use super::DialContext;
use super::{CtxResult, Message, MessageId, Select, UserId};
use anyhow::Result;
use sec_store::repository::{RecordsRepository, RepositoryOpenError};

pub struct OpenRepoDialogue<F, R> {
    factory: F,
    user_id: UserId,
    sent_msg_ids: HashSet<MessageId>,
    //
    phantom: PhantomData<R>,
}

impl<F, R> OpenRepoDialogue<F, R> {
    pub fn new(factory: F, user_id: UserId) -> Self {
        OpenRepoDialogue {
            factory,
            user_id,
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
            self.sent_msg_ids
                .clone()
                .into_iter()
                .map(|msg_id| {
                    self.sent_msg_ids.remove(&msg_id);
                    msg_id
                })
                .collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        Ok(vec![select
            .msg_id
            .map(|msg_id| CtxResult::RemoveMessages(vec![msg_id]))
            .unwrap_or(CtxResult::Nothing)])
    }

    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match (&message.user_id, message.text) {
            (Some(user_id), Some(passwd)) => {
                match self
                    .factory
                    .get_user_repository(&user_id.clone().into(), passwd)
                {
                    Ok(repo) => CtxResult::NewCtx(Box::new(ViewRepoDialog::new(repo))),
                    Err(RepositoryOpenError::WrongPassword) => {
                        CtxResult::Messages(
                            vec!["Пароль не подходит 🤨. Попробуйте еще раз".into()],
                        )
                    }
                    Err(RepositoryOpenError::DoesntExist) => CtxResult::NewCtx(Box::new(
                        HelloDialogue::new(user_id.clone(), self.factory.clone()),
                    )),
                    Err(RepositoryOpenError::UnexpectedError) => {
                        log::error!("Unexpected error an attempt open repository for {user_id}");
                        CtxResult::Messages(vec![
                            "Не удалось открыть репозиторий. Непредвиденная ошибка 🥵".into(),
                        ])
                    }
                }
            }
            _ => CtxResult::Nothing,
        };
        Ok(vec![CtxResult::RemoveMessages(vec![message.id]), result])
    }

    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![command.id])])
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
}
