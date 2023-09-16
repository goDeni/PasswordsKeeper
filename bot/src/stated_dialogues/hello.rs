use std::{collections::HashSet, sync::Arc};

use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;

use super::{
    create_repo::CreateRepoDialogue, ButtonPayload, CtxResult, DialContext, DialogState,
    UserId, Message, MessageId, Select, open_repo::OpenRepoDialogue,
};

pub struct HelloDialogue<T> {
    user_id: UserId,
    factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    state: DialogState,
    sent_msg_ids: HashSet<MessageId>,
}

impl<T> HelloDialogue<T>
where
    T: RecordsRepository,
{
    pub fn new(
        user_id: UserId,
        factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    ) -> Self {
        HelloDialogue {
            user_id,
            factory,
            state: DialogState::IDLE,
            sent_msg_ids: HashSet::new(),
        }
    }
}

const CREATE_REPO: &'static str = "1";
const OPEN_REPO: &'static str = "2";

impl<T> DialContext for HelloDialogue<T>
where
    T: RecordsRepository + Sync + Send + 'static,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
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
        let result: CtxResult = match (&select.user_id, select.data()) {
            (user_id, Some(OPEN_REPO)) => CtxResult::NewCtx(Box::new(OpenRepoDialogue::new(
                self.factory.clone(),
                user_id.clone(),
            ))),
            (_, Some(CREATE_REPO)) => CtxResult::NewCtx(Box::new(CreateRepoDialogue::new(
                self.user_id.clone(),
                self.factory.clone(),
            ))),
            _ => CtxResult::Nothing
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

    fn handle_message(&mut self, input: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![input.id])])
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sec_store::repository::RecordsRepository;
    use tempdir::TempDir;

    use crate::{
        stated_dialogues::{
            hello::{CREATE_REPO, OPEN_REPO},
            ButtonPayload, MessageId, Select, UserId,
        },
        user_repo_factory::{file::FileRepositoriesFactory, RepositoriesFactory},
    };

    use super::{CtxResult, DialContext, HelloDialogue};

    #[test]
    fn test_init_no_rep() {
        let dial_id = super::UserId("1".to_string());
        let tmp_dir = TempDir::new("tests_").unwrap();
        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        let mut dialog = HelloDialogue::new(dial_id, Arc::new(Box::new(factory)));
        let mut result = dialog.init().unwrap();
        assert_eq!(result.len(), 1);

        match result.remove(0) {
            CtxResult::Buttons(message, selector) => {
                assert_eq!(message, "Выберите действие".into());
                assert_eq!(
                    selector,
                    vec![vec![(
                        ButtonPayload(CREATE_REPO.to_string()),
                        "Создать репозиторий".to_string()
                    )]]
                );
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn test_init_with_rep() {
        let dial_id = super::UserId("1".to_string());
        let passwd = "123".to_string();

        let tmp_dir = TempDir::new("tests_").unwrap();
        let factory = FileRepositoriesFactory(tmp_dir.into_path());
        let rep = factory
            .initialize_user_repository(&dial_id.0, passwd)
            .unwrap();
        rep.save().unwrap();

        let mut dialog = HelloDialogue::new(dial_id, Arc::new(Box::new(factory)));
        let mut result = dialog.init().unwrap();
        assert_eq!(result.len(), 1);

        match result.remove(0) {
            CtxResult::Buttons(message, selector) => {
                assert_eq!(message, "Репозиторий".into());
                assert_eq!(
                    selector,
                    vec![vec![(
                        ButtonPayload(OPEN_REPO.to_string()),
                        "Открыть репозиторий".to_string()
                    )]]
                );
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn test_create_repo_select() {
        let dial_id = super::UserId("1".to_string());
        let tmp_dir = TempDir::new("tests_").unwrap();
        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        let mut dialog = HelloDialogue::new(dial_id, Arc::new(Box::new(factory)));

        dialog.init().unwrap();
        let mut result = dialog
            .handle_select(Select::new(
                Some(MessageId(1)),
                Some(CREATE_REPO.to_string()),
                UserId("1".into()),
            ))
            .unwrap();

        match result.remove(0) {
            CtxResult::NewCtx(mut ctx) => {
                ctx.init().unwrap();
            }
            _ => assert!(false),
        }
    }
}
