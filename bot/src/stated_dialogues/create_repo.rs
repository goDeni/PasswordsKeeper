use std::sync::Arc;

use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;

use super::{CtxResult, DialContext, DialogState, DialogueId, Message};

#[derive(Clone)]
enum CreationState {
    Disabled,
    WaitForPassword,
    WaitPasswordRepeat(String),
}

pub struct CreateRepoDialogue<T>
where
    T: RecordsRepository,
{
    dial_id: DialogueId,
    factory: Arc<Box<dyn RepositoriesFactory<T> + Sync + Send>>,
    state: DialogState,
    creation_state: CreationState,
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
            state: DialogState::IDLE,
            creation_state: CreationState::Disabled,
        }
    }
}

impl<T> DialContext for CreateRepoDialogue<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> Result<super::CtxResult> {
        self.creation_state = CreationState::WaitForPassword;
        Ok(CtxResult::Messages(vec!["Придумайте пароль".to_string()]))
    }

    fn shutdown(&self) -> Result<super::CtxResult> {
        Ok(CtxResult::Nothing)
    }

    fn handle_select(&mut self, _select: &str) -> Result<CtxResult> {
        Ok(CtxResult::Nothing)
    }

    fn handle_message(&mut self, input: Message) -> Result<CtxResult> {
        let input = input.text().unwrap_or("");

        match self.creation_state.clone() {
            CreationState::WaitForPassword => {
                if input.is_empty() {
                    return Ok(CtxResult::Messages(vec!["Вы ничего не ввели!".to_string()]));
                }
                self.creation_state = CreationState::WaitPasswordRepeat(input.to_string());
                Ok(CtxResult::Messages(vec!["Повторите пароль".to_string()]))
            }
            CreationState::WaitPasswordRepeat(passwd) => {
                if passwd.ne(input) {
                    return Ok(CtxResult::Messages(vec![
                        "Неверный пароль. Попробуйте еще раз".to_string(),
                    ]));
                }
                self.creation_state = CreationState::Disabled;
                Ok(CtxResult::Messages(vec!["Все четко!".to_string()]))
            }
            _ => Ok(CtxResult::Messages(vec!["?".to_string()])),
        }
    }

    fn handle_command(&mut self, _command: &str) -> Result<CtxResult> {
        todo!()
    }
}
