use std::sync::Arc;

use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::RepositoriesFactory;

use super::{DialContext, DialogueId, State, CtxResult};

pub struct CreateRepoDialogue<T>
where
    T: RecordsRepository,
{
    dial_id: DialogueId,
    factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    state: State,
}

impl<T> CreateRepoDialogue<T>
where
    T: RecordsRepository,
{
    pub fn new(
        dial_id: DialogueId,
        factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    ) -> Self {
        CreateRepoDialogue {
            dial_id,
            factory,
            state: State::IDLE,
        }
    }
}

impl<T> DialContext for CreateRepoDialogue<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> anyhow::Result<super::CtxResult> {
        self.state = State::WaitForInput;
        Ok(CtxResult::Messages(vec![
            "Придумайте пароль".to_string()
        ]))
    }

    fn shutdown(&self) -> anyhow::Result<super::CtxResult> {
        todo!()
    }

    fn handle_select(&mut self, _select: &str) -> anyhow::Result<super::CtxResult> {
        todo!()
    }

    fn handle_input(&mut self, _input: &str) -> anyhow::Result<super::CtxResult> {
        match self.state {
            State::WaitForInput => Ok(super::CtxResult::Messages(vec![
                "Введен пароль!".to_string()
            ])),
            _ => Ok(super::CtxResult::Messages(vec!["?".to_string()])),
        }
    }

    fn handle_command<C>(&mut self, _command: C) -> anyhow::Result<super::CtxResult>
    where
        C: AsRef<str>,
        Self: Sized,
    {
        todo!()
    }
}
