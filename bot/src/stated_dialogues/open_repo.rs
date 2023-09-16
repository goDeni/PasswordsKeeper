use std::collections::HashSet;
use std::sync::Arc;

use crate::user_repo_factory::RepositoriesFactory;

use super::hello::HelloDialogue;
use super::view_repo::ViewRepo;
use super::DialContext;
use super::{CtxResult, Message, MessageId, Select, UserId};
use anyhow::Result;
use sec_store::repository::{RecordsRepository, RepositoryOpenError};

pub struct OpenRepoDialogue<T>
where
    T: RecordsRepository,
{
    factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    user_id: UserId,
    sent_msg_ids: HashSet<MessageId>,
}

impl<T> OpenRepoDialogue<T>
where
    T: RecordsRepository,
{
    pub fn new(
        factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
        user_id: UserId,
    ) -> Self {
        OpenRepoDialogue {
            factory,
            user_id,
            sent_msg_ids: HashSet::new(),
        }
    }
}

impl<T> DialContext for OpenRepoDialogue<T>
where
    T: RecordsRepository + Sync + Send + 'static,
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
                    Ok(repo) => CtxResult::NewCtx(Box::new(ViewRepo::new(repo))),
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
