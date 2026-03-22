use ratatui::symbols::border;
use ratatui::{layout::Rect, widgets::Block, Frame};
use sec_store::repository::RecordsRepository;

use crate::dialogues::{Dialogue, DialogueResult};
use crate::repo::RepositoryFactory;

#[derive(Debug)]
pub struct CreateRepoDialogue<F> {
    factory: F,
    step: CreateRepoStep,
    first_password: String,
}

#[derive(Clone, Debug)]
pub enum CreateRepoStep {
    Password1,
    Password2,
}

impl<F> CreateRepoDialogue<F> {
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            step: CreateRepoStep::Password1,
            first_password: String::new(),
        }
    }

    pub fn set_first_password(&mut self, pwd: String) {
        self.first_password = pwd;
        self.step = CreateRepoStep::Password2;
    }
}

impl<F, R> Dialogue<F, R> for CreateRepoDialogue<F>
where
    F: RepositoryFactory<R>,
    R: RecordsRepository,
{
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Create repository ")
            .border_set(border::ROUNDED);
        frame.render_widget(block, area);
    }

    fn handle_key(&mut self, _k: crossterm::event::KeyEvent) -> DialogueResult<F, R> {
        DialogueResult::NoOp
    }

    fn on_input_submit(&mut self, value: String) -> DialogueResult<F, R> {
        match self.step {
            CreateRepoStep::Password1 => {
                if value.is_empty() {
                    return DialogueResult::Error("Password cannot be empty".to_string());
                }
                self.set_first_password(value);
                DialogueResult::StartInput {
                    prompt: "Repeat the password".to_string(),
                    password: true,
                }
            }
            CreateRepoStep::Password2 => {
                if value != self.first_password {
                    return DialogueResult::StartInput {
                        prompt: "Repeat the password (wrong password)".to_string(),
                        password: true,
                    };
                }
                match self.factory.create_repo(self.first_password.clone()) {
                    Ok(repo) => DialogueResult::ChangeScreen(Box::new(
                        crate::dialogues::view_repo::ViewRepoDialogue::new(
                            self.factory.clone(),
                            repo,
                            Some(0),
                        ),
                    )),
                    Err(e) => DialogueResult::Error(e.to_string()),
                }
            }
        }
    }

    fn on_input_cancel(&mut self) -> DialogueResult<F, R> {
        DialogueResult::ChangeScreen(Box::new(crate::dialogues::welcome::WelcomeDialogue::new(
            self.factory.clone(),
            Some(0),
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::dialogues::{Dialogue, DialogueResult};
    use crate::repo::{FileRepositoryFactory, RepositoryFactory};
    use crate::test_helpers::{test_password, ScopedTuiDataDir};

    use super::{CreateRepoDialogue, CreateRepoStep};

    #[test]
    fn test_empty_first_password_returns_error() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = CreateRepoDialogue::new(factory);
        let res = dialogue.on_input_submit(String::new());

        match res {
            DialogueResult::Error(msg) => assert_eq!(msg, "Password cannot be empty"),
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn test_first_password_moves_to_repeat_step() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = CreateRepoDialogue::new(factory);
        let res = dialogue.on_input_submit(test_password());

        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Repeat the password");
                assert!(password);
            }
            _ => panic!("expected StartInput"),
        }
        assert!(matches!(dialogue.step, CreateRepoStep::Password2));
    }

    #[test]
    fn test_repeat_password_mismatch_reprompts() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = CreateRepoDialogue::new(factory);
        let _ = dialogue.on_input_submit(test_password());
        let res = dialogue.on_input_submit("wrong".to_string());

        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Repeat the password (wrong password)");
                assert!(password);
            }
            _ => panic!("expected StartInput"),
        }
    }

    #[test]
    fn test_create_repo_success_changes_screen() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = CreateRepoDialogue::new(factory.clone());
        let password = test_password();
        let _ = dialogue.on_input_submit(password.clone());
        let res = dialogue.on_input_submit(password);

        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
        assert!(factory.has_repo());
    }

    #[test]
    fn test_cancel_returns_to_welcome() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = CreateRepoDialogue::new(factory);
        let res = dialogue.on_input_cancel();
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }
}
