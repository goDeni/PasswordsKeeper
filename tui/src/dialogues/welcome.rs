use crossterm::event::KeyCode;
use ratatui::symbols::border;
use ratatui::widgets::Block;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};
use sec_store::repository::RecordsRepository;

use crate::dialogues::create_repo::CreateRepoDialogue;
use crate::dialogues::open_repo::OpenRepoDialogue;
use crate::dialogues::{Dialogue, DialogueResult};
use crate::repo::RepositoryFactory;

#[derive(Debug)]
pub struct WelcomeDialogue<F> {
    factory: F,
    list_state: ListState,
}

impl<F> WelcomeDialogue<F> {
    pub fn new(factory: F, selected: Option<usize>) -> Self {
        let mut state = ListState::default();
        state.select(selected);
        Self {
            factory,
            list_state: state,
        }
    }
}

impl<F, R> Dialogue<F, R> for WelcomeDialogue<F>
where
    F: RepositoryFactory<R>,
    R: RecordsRepository,
{
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" PasswordsKeeper ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().cyan());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut items = vec![
            ListItem::new("Create repository"),
            ListItem::new("Open repository"),
            ListItem::new("Quit"),
        ];
        if !self.factory.has_repo() {
            items[1] =
                ListItem::new("Open repository (none exists)").style(Style::new().dark_gray());
        }
        let list = List::new(items)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, inner, &mut self.list_state);

        let instructions = Line::from(vec![
            Span::raw(" "),
            Span::styled("↑/k", Style::new().cyan()),
            Span::raw(" up "),
            Span::styled("↓/j", Style::new().cyan()),
            Span::raw(" down "),
            Span::styled("Enter", Style::new().cyan()),
            Span::raw(" select "),
            Span::styled("q", Style::new().cyan()),
            Span::raw(" quit"),
        ]);
        let bottom = Rect {
            y: area.y + area.height.saturating_sub(1),
            ..area
        };
        frame.render_widget(
            Paragraph::new(instructions).style(Style::new().dim()),
            bottom,
        );
    }

    fn handle_key(&mut self, key_event: crossterm::event::KeyEvent) -> DialogueResult<F, R> {
        let n = 3;
        let sel = self.list_state.selected().unwrap_or(0);
        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state
                    .select(Some(if sel == 0 { n - 1 } else { sel - 1 }));
                DialogueResult::NoOp
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select(Some((sel + 1) % n));
                DialogueResult::NoOp
            }
            KeyCode::Enter => match sel {
                0 => DialogueResult::ChangeScreenAndStartInput {
                    dialogue: Box::new(CreateRepoDialogue::new(self.factory.clone())),
                    prompt: "Choose a password".to_string(),
                    password: true,
                },
                1 => {
                    if self.factory.has_repo() {
                        DialogueResult::ChangeScreenAndStartInput {
                            dialogue: Box::new(OpenRepoDialogue::new(self.factory.clone())),
                            prompt: "Enter password".to_string(),
                            password: true,
                        }
                    } else {
                        DialogueResult::NoOp
                    }
                }
                2 => DialogueResult::Exit,
                _ => DialogueResult::NoOp,
            },
            _ => DialogueResult::NoOp,
        }
    }

    fn on_input_submit(&mut self, _value: String) -> DialogueResult<F, R> {
        DialogueResult::NoOp
    }

    fn on_input_cancel(&mut self) -> DialogueResult<F, R> {
        DialogueResult::NoOp
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::dialogues::{Dialogue, DialogueResult};
    use crate::repo::{FileRepositoryFactory, RepositoryFactory};
    use crate::test_helpers::{test_password, ScopedTuiDataDir};

    use super::WelcomeDialogue;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_navigation_wraps_up() {
        let scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(scope.temp_dir.path().join("repo"));
        let mut dialogue = WelcomeDialogue::new(factory, Some(0));
        let res = dialogue.handle_key(key(KeyCode::Up));
        assert!(matches!(res, DialogueResult::NoOp));
        assert_eq!(dialogue.list_state.selected(), Some(2));
    }

    #[test]
    fn test_navigation_wraps_down() {
        let scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(scope.temp_dir.path().join("repo"));
        let mut dialogue = WelcomeDialogue::new(factory, Some(2));
        let res = dialogue.handle_key(key(KeyCode::Down));
        assert!(matches!(res, DialogueResult::NoOp));
        assert_eq!(dialogue.list_state.selected(), Some(0));
    }

    #[test]
    fn test_enter_create_starts_password_input() {
        let scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(scope.temp_dir.path().join("repo"));
        let mut dialogue = WelcomeDialogue::new(factory, Some(0));

        let res = dialogue.handle_key(key(KeyCode::Enter));
        match res {
            DialogueResult::ChangeScreenAndStartInput {
                prompt, password, ..
            } => {
                assert_eq!(prompt, "Choose a password");
                assert!(password);
            }
            _ => panic!("expected ChangeScreenAndStartInput"),
        }
    }

    #[test]
    fn test_enter_open_without_repo_is_noop() {
        let scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(scope.temp_dir.path().join("repo"));
        let mut dialogue = WelcomeDialogue::new(factory, Some(1));

        let res = dialogue.handle_key(key(KeyCode::Enter));
        assert!(matches!(res, DialogueResult::NoOp));
    }

    #[test]
    fn test_enter_open_with_repo_starts_password_input() {
        let scope = ScopedTuiDataDir::new();
        let repo_path = scope.temp_dir.path().join("repo");
        let factory = FileRepositoryFactory::new(repo_path);
        factory
            .create_repo(test_password())
            .expect("repo creation failed");
        let mut dialogue = WelcomeDialogue::new(factory, Some(1));

        let res = dialogue.handle_key(key(KeyCode::Enter));
        match res {
            DialogueResult::ChangeScreenAndStartInput {
                prompt, password, ..
            } => {
                assert_eq!(prompt, "Enter password");
                assert!(password);
            }
            _ => panic!("expected ChangeScreenAndStartInput"),
        }
    }

    #[test]
    fn test_enter_quit_returns_exit() {
        let scope = ScopedTuiDataDir::new();
        let factory = FileRepositoryFactory::new(scope.temp_dir.path().join("repo"));
        let mut dialogue = WelcomeDialogue::new(factory, Some(2));
        let res = dialogue.handle_key(key(KeyCode::Enter));
        assert!(matches!(res, DialogueResult::Exit));
    }
}
