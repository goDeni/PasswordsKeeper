use ratatui::symbols::border;
use ratatui::{layout::Rect, widgets::Block, Frame};

use crate::dialogues::{Dialogue, DialogueResult};

#[derive(Debug)]
pub struct CreateRepoDialogue {
    step: CreateRepoStep,
    first_password: String,
}

#[derive(Clone, Debug)]
pub enum CreateRepoStep {
    Password1,
    Password2,
}

impl CreateRepoDialogue {
    pub fn new() -> Self {
        Self {
            step: CreateRepoStep::Password1,
            first_password: String::new(),
        }
    }

    pub fn set_first_password(&mut self, pwd: String) {
        self.first_password = pwd;
        self.step = CreateRepoStep::Password2;
    }
}

impl Dialogue for CreateRepoDialogue {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Create repository ")
            .border_set(border::ROUNDED);
        frame.render_widget(block, area);
    }

    fn handle_key(&mut self, _k: crossterm::event::KeyEvent) -> DialogueResult {
        DialogueResult::NoOp
    }

    fn on_input_submit(&mut self, value: String) -> DialogueResult {
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
                match crate::repo::create_repo(self.first_password.clone()) {
                    Ok(repo) => DialogueResult::ChangeScreen(Box::new(
                        crate::dialogues::view_repo::ViewRepoDialogue::new(repo, Some(0)),
                    )),
                    Err(e) => DialogueResult::Error(e.to_string()),
                }
            }
        }
    }

    fn on_input_cancel(&mut self) -> DialogueResult {
        DialogueResult::ChangeScreen(Box::new(crate::dialogues::welcome::WelcomeDialogue::new(
            Some(0),
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::dialogues::{Dialogue, DialogueResult};
    use crate::repo;
    use crate::test_helpers::ScopedTuiDataDir;

    use super::{CreateRepoDialogue, CreateRepoStep};

    #[test]
    fn test_empty_first_password_returns_error() {
        let _scope = ScopedTuiDataDir::new();
        let mut dialogue = CreateRepoDialogue::new();
        let res = dialogue.on_input_submit(String::new());

        match res {
            DialogueResult::Error(msg) => assert_eq!(msg, "Password cannot be empty"),
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn test_first_password_moves_to_repeat_step() {
        let _scope = ScopedTuiDataDir::new();
        let mut dialogue = CreateRepoDialogue::new();
        let res = dialogue.on_input_submit("pass".to_string());

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
        let mut dialogue = CreateRepoDialogue::new();
        let _ = dialogue.on_input_submit("pass".to_string());
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
        let mut dialogue = CreateRepoDialogue::new();
        let _ = dialogue.on_input_submit("pass".to_string());
        let res = dialogue.on_input_submit("pass".to_string());

        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
        assert!(repo::has_repo());
    }

    #[test]
    fn test_cancel_returns_to_welcome() {
        let _scope = ScopedTuiDataDir::new();
        let mut dialogue = CreateRepoDialogue::new();
        let res = dialogue.on_input_cancel();
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }
}
