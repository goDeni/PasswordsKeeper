use std::sync::Arc;

use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;

use super::{
    create_repo::CreateRepoDialogue, ButtonPayload, CtxResult, DialContext, DialogState,
    DialogueId, Message,
};

pub struct HelloDialogue<T> {
    dial_id: DialogueId,
    factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    state: DialogState,
}

impl<T> HelloDialogue<T>
where
    T: RecordsRepository,
{
    pub fn new(
        dial_id: DialogueId,
        factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    ) -> Self {
        HelloDialogue {
            dial_id,
            factory,
            state: DialogState::IDLE,
        }
    }
}

const CREATE_REPO: &'static str = "1";
const OPEN_REPO: &'static str = "2";

impl<T> DialContext for HelloDialogue<T>
where
    T: RecordsRepository + 'static,
{
    fn init(&mut self) -> Result<CtxResult> {
        match self.factory.user_has_repository(&self.dial_id.0) {
            false => Ok(CtxResult::Buttons(
                "Выберите действие".to_string(),
                vec![vec![(
                    ButtonPayload(CREATE_REPO.to_string()),
                    "Создать репозиторий".to_string(),
                )]],
            )),
            true => Ok(CtxResult::Buttons(
                "Репозиторий".to_string(),
                vec![vec![(
                    ButtonPayload(OPEN_REPO.to_string()),
                    "Открыть репозиторий".to_string(),
                )]],
            )),
        }
    }

    fn shutdown(&self) -> Result<CtxResult> {
        Ok(CtxResult::Nothing)
    }

    fn handle_select(&mut self, select: &str) -> Result<CtxResult> {
        match select.as_ref() {
            OPEN_REPO => Ok(CtxResult::Nothing),
            CREATE_REPO => Ok(CtxResult::NewCtx(Box::new(CreateRepoDialogue::new(
                self.dial_id.clone(),
                self.factory.clone(),
            )))),
            _ => Ok(CtxResult::Nothing),
        }
    }

    fn handle_message(&mut self, input: Message) -> Result<CtxResult> {
        Ok(CtxResult::RemoveMessages(vec![input.id().to_owned()]))
    }

    fn handle_command(&mut self, _command: &str) -> Result<CtxResult> {
        Ok(CtxResult::Nothing)
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
            ButtonPayload,
        },
        user_repo_factory::{file::FileRepositoriesFactory, RepositoriesFactory},
    };

    use super::{CtxResult, DialContext, HelloDialogue};

    #[test]
    fn test_init_no_rep() {
        let dial_id = super::DialogueId("1".to_string());
        let tmp_dir = TempDir::new("tests_").unwrap();
        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        let mut dialog = HelloDialogue::new(dial_id, Arc::new(Box::new(factory)));

        match dialog.init().unwrap() {
            CtxResult::Buttons(message, selector) => {
                assert_eq!(message, "Выберите действие".to_string());
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
        let dial_id = super::DialogueId("1".to_string());
        let passwd = "123".to_string();

        let tmp_dir = TempDir::new("tests_").unwrap();
        let factory = FileRepositoriesFactory(tmp_dir.into_path());
        let rep = factory
            .initialize_user_repository(&dial_id.0, passwd)
            .unwrap();
        rep.save().unwrap();

        let mut dialog = HelloDialogue::new(dial_id, Arc::new(Box::new(factory)));

        match dialog.init().unwrap() {
            CtxResult::Buttons(message, selector) => {
                assert_eq!(message, "Репозиторий".to_string());
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
        let dial_id = super::DialogueId("1".to_string());
        let tmp_dir = TempDir::new("tests_").unwrap();
        let factory = FileRepositoriesFactory(tmp_dir.into_path());

        let mut dialog = HelloDialogue::new(dial_id, Arc::new(Box::new(factory)));

        dialog.init().unwrap();

        match dialog.handle_select(CREATE_REPO).unwrap() {
            CtxResult::NewCtx(mut ctx) => {
                ctx.init().unwrap();
            }
            _ => assert!(false),
        }
    }
}
