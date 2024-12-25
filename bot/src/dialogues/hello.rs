use std::{collections::HashSet, marker::PhantomData};

use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;

use stated_dialogues::dialogues::{
    ButtonPayload, CtxResult, DialContext, Message, MessageId, Select, UserId,
};

use super::repository::{create_repo::CreateRepoDialogue, open_repo::OpenRepoDialogue};
use async_trait::async_trait;

pub struct HelloDialogue<T, R> {
    user_id: UserId,
    factory: T,
    sent_msg_ids: HashSet<MessageId>,
    //
    phantom: PhantomData<R>,
}

impl<T, R> HelloDialogue<T, R> {
    pub fn new(user_id: UserId, factory: T) -> Self {
        HelloDialogue {
            user_id,
            factory,
            sent_msg_ids: HashSet::new(),
            //
            phantom: PhantomData,
        }
    }
}

const CREATE_REPO: &str = "1";
const OPEN_REPO: &str = "2";

#[async_trait]
impl<F, R> DialContext for HelloDialogue<F, R>
where
    R: RecordsRepository,
    F: RepositoriesFactory<R>,
{
    async fn init(&mut self) -> Result<Vec<CtxResult>> {
        match self.factory.user_has_repository(&self.user_id.0) {
            false => Ok(vec![CtxResult::Buttons(
                "Выберите действие".into(),
                vec![vec![(
                    ButtonPayload(CREATE_REPO.to_string()),
                    "Создать репозиторий".to_string(),
                )]],
            )]),
            true => Ok(vec![CtxResult::Buttons(
                "Репозиторий".into(),
                vec![vec![(
                    ButtonPayload(OPEN_REPO.to_string()),
                    "Открыть репозиторий".to_string(),
                )]],
            )]),
        }
    }

    async fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    async fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match (&select.user_id, select.data()) {
            (_, Some(OPEN_REPO)) => {
                CtxResult::NewCtx(Box::new(OpenRepoDialogue::new(self.factory.clone())))
            }
            (_, Some(CREATE_REPO)) => {
                CtxResult::NewCtx(Box::new(CreateRepoDialogue::new(self.factory.clone())))
            }
            _ => CtxResult::Nothing,
        };

        if let Some(msg_id) = &select.msg_id {
            self.sent_msg_ids.remove(msg_id);
        }

        Ok(vec![
            result,
            select
                .msg_id
                .map(|msg_id| CtxResult::RemoveMessages(vec![msg_id]))
                .unwrap_or(CtxResult::Nothing),
        ])
    }

    async fn handle_message(&mut self, input: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![input.id])])
    }

    async fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![command.id])])
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
}
