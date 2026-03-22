use ratatui::symbols::border;
use ratatui::{layout::Rect, widgets::Block, Frame};
use sec_store::repository::RecordsRepository;

use crate::dialogues::{Dialogue, DialogueResult};
use crate::repo::RepositoryFactory;

#[derive(Debug)]
pub struct OpenRepoDialogue<F> {
    factory: F,
}

impl<F> OpenRepoDialogue<F> {
    pub fn new(factory: F) -> Self {
        Self { factory }
    }
}

impl<F, R> Dialogue<F, R> for OpenRepoDialogue<F>
where
    F: RepositoryFactory<R>,
    R: RecordsRepository,
{
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Open repository ")
            .border_set(border::ROUNDED);
        frame.render_widget(block, area);
    }

    fn handle_key(&mut self, _k: crossterm::event::KeyEvent) -> DialogueResult<F, R> {
        DialogueResult::NoOp
    }

    fn on_input_submit(&mut self, value: String) -> DialogueResult<F, R> {
        match self.factory.open_repo(value) {
            Ok(repo) => DialogueResult::ChangeScreen(Box::new(
                crate::dialogues::view_repo::ViewRepoDialogue::new(
                    self.factory.clone(),
                    repo,
                    Some(0),
                ),
            )),
            Err(e) => {
                // On error, show the error and restart input so user can try again
                DialogueResult::StartInput {
                    prompt: format!("Enter password ({})", e),
                    password: true,
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

    use super::OpenRepoDialogue;

    #[test]
    fn test_open_repo_success_changes_screen() {
        let _scope = ScopedTuiDataDir::new();
        let password = test_password();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        factory
            .create_repo(password.clone())
            .expect("repo creation failed");

        let mut dialogue = OpenRepoDialogue::new(factory);
        let res = dialogue.on_input_submit(password);

        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_open_repo_wrong_password_restarts_input() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        factory
            .create_repo(test_password())
            .expect("repo creation failed");
        let wrong_password = test_password();

        let mut dialogue = OpenRepoDialogue::new(factory);
        let res = dialogue.on_input_submit(wrong_password);

        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert!(prompt.contains("Wrong password"));
                assert!(password);
            }
            _ => panic!("expected StartInput"),
        }
    }

    #[test]
    fn test_open_repo_missing_repository_restarts_input() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = OpenRepoDialogue::new(factory);
        let res = dialogue.on_input_submit(test_password());

        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert!(prompt.contains("Repository does not exist"));
                assert!(password);
            }
            _ => panic!("expected StartInput"),
        }
    }

    #[test]
    fn test_cancel_returns_to_welcome() {
        let _scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(_scope.temp_dir.path().join("repo"));
        let mut dialogue = OpenRepoDialogue::new(factory);
        let res = dialogue.on_input_cancel();
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }
}
