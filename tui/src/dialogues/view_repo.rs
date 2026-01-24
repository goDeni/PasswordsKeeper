use crate::dialogues::{AddRecordDialogue, ViewRecordDialogue, WelcomeDialogue};
use crossterm::event::KeyCode;
use ratatui::symbols::border;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};
use sec_store::repository::file::RecordsFileRepository;
use sec_store::repository::RecordsRepository;

use crate::dialogues::{Dialogue, DialogueResult};
use crate::fields::{RECORD_LOGIN_FIELD, RECORD_NAME_FIELD};

type RecordId = String;

#[derive(Debug)]
pub struct ViewRepoDialogue {
    repo: RecordsFileRepository,
    list_state: ListState,
    search_query: String,
    is_searching: bool,
}

impl ViewRepoDialogue {
    pub fn new(repo: RecordsFileRepository, selected: Option<usize>) -> Self {
        let mut state = ListState::default();
        state.select(selected);
        Self {
            repo,
            list_state: state,
            search_query: String::new(),
            is_searching: false,
        }
    }

    fn get_filtered_records(&self) -> Vec<(RecordId, String)> {
        let records = self.repo.get_records().unwrap_or_default();
        // Collect records with both name and login for filtering
        let mut rows: Vec<(RecordId, String, Option<String>)> = records
            .iter()
            .map(|r| {
                let name = r
                    .get_field_value(RECORD_NAME_FIELD)
                    .unwrap_or_else(|| "-".to_string());
                let login = r.get_field_value(RECORD_LOGIN_FIELD);
                (r.id.clone(), name, login)
            })
            .collect();

        // Filter by search query if searching
        if self.is_searching && !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            rows.retain(|(_, name, login)| {
                let name_match = name.to_lowercase().contains(&query_lower);
                let login_match = login
                    .as_ref()
                    .map(|l| l.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);
                name_match || login_match
            });
        }

        rows.sort_by(|a, b| a.1.cmp(&b.1));
        // Convert back to (RecordId, String) format
        rows.into_iter().map(|(id, name, _)| (id, name)).collect()
    }
}

impl Dialogue for ViewRepoDialogue {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Repository ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().green());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = self.get_filtered_records();

        let mut items: Vec<ListItem> = rows
            .iter()
            .map(|(_, name)| ListItem::new(name.as_str()))
            .collect();
        items.push(ListItem::new("─── Add record"));
        items.push(ListItem::new("─── Close repository"));

        // Adjust selection to valid range
        let sel = self.list_state.selected().unwrap_or(0);
        let max_sel = items.len().saturating_sub(1);
        if sel > max_sel {
            self.list_state.select(Some(max_sel));
        }

        let list = List::new(items.clone())
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        state.select(self.list_state.selected());
        frame.render_stateful_widget(list, inner, &mut state);

        // Draw search bar if searching
        if self.is_searching {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            let search_text = format!("/{}", self.search_query);
            let search_para = Paragraph::new(search_text.as_str())
                .style(Style::new().yellow())
                .block(Block::default());
            frame.render_widget(search_para, chunks[1]);
        }

        let instructions = if self.is_searching {
            Line::from(vec![
                Span::styled("Esc", Style::new().cyan()),
                Span::raw(" cancel search "),
                Span::styled("↑/↓", Style::new().cyan()),
                Span::raw(" navigate "),
                Span::styled("Enter", Style::new().cyan()),
                Span::raw(" view "),
            ])
        } else {
            Line::from(vec![
                Span::styled("/", Style::new().cyan()),
                Span::raw(" search "),
                Span::styled("Enter", Style::new().cyan()),
                Span::raw(" view "),
                Span::styled("a", Style::new().cyan()),
                Span::raw(" add "),
                Span::styled("c", Style::new().cyan()),
                Span::raw(" close "),
                Span::styled("q", Style::new().cyan()),
                Span::raw(" quit"),
            ])
        };
        let bottom = Rect {
            y: area.y + area.height.saturating_sub(1),
            ..area
        };
        frame.render_widget(
            Paragraph::new(instructions).style(Style::new().dim()),
            bottom,
        );
    }

    fn handle_key(&mut self, k: crossterm::event::KeyEvent) -> DialogueResult {
        // Handle search mode input
        if self.is_searching {
            match k.code {
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.search_query.clear();
                    self.list_state.select(Some(0));
                    return DialogueResult::NoOp;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.list_state.select(Some(0)); // Reset to first item when filtering
                    return DialogueResult::NoOp;
                }
                KeyCode::Enter => {
                    // Exit search mode but keep filter, navigate to selected item
                    self.is_searching = false;
                    // Continue to handle Enter normally below
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.list_state.select(Some(0)); // Reset to first item when filtering
                    return DialogueResult::NoOp;
                }
                _ => {
                    // Allow arrow keys to work during search
                }
            }
        } else {
            // Start search mode
            if k.code == KeyCode::Char('/') {
                self.is_searching = true;
                self.search_query.clear();
                return DialogueResult::NoOp;
            }
        }

        let rows = self.get_filtered_records();
        let n_rec = rows.len();
        let n = n_rec + 2; // Add, Close
        let sel = self
            .list_state
            .selected()
            .unwrap_or(0)
            .min(n.saturating_sub(1));

        match k.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state
                    .select(Some(if sel == 0 { n - 1 } else { sel - 1 }));
                DialogueResult::NoOp
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select(Some((sel + 1) % n));
                DialogueResult::NoOp
            }
            KeyCode::Char('a') if !self.is_searching => DialogueResult::ChangeScreenAndStartInput {
                dialogue: Box::new(crate::dialogues::add_record::AddRecordDialogue::new(
                    self.repo.clone(),
                )),
                prompt: "Enter password".to_string(),
                password: true,
            },
            KeyCode::Char('c') if !self.is_searching => DialogueResult::ChangeScreen(Box::new(
                crate::dialogues::welcome::WelcomeDialogue::new(Some(0)),
            )),
            KeyCode::Enter => {
                if sel < n_rec {
                    let rid = rows[sel].0.clone();
                    DialogueResult::ChangeScreen(Box::new(ViewRecordDialogue::new(
                        self.repo.clone(),
                        rid,
                        false,
                    )))
                } else if sel == n_rec {
                    DialogueResult::ChangeScreenAndStartInput {
                        dialogue: Box::new(AddRecordDialogue::new(self.repo.clone())),
                        prompt: "Enter password".to_string(),
                        password: true,
                    }
                } else {
                    DialogueResult::ChangeScreen(Box::new(WelcomeDialogue::new(Some(0))))
                }
            }
            _ => DialogueResult::NoOp,
        }
    }

    fn on_input_submit(&mut self, _value: String) -> DialogueResult {
        DialogueResult::NoOp
    }

    fn on_input_cancel(&mut self) -> DialogueResult {
        DialogueResult::NoOp
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tempfile::TempDir;

    use crate::dialogues::{Dialogue, DialogueResult};
    use crate::fields::{RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD};
    use sec_store::record::Record;
    use sec_store::repository::file::RecordsFileRepository;
    use sec_store::repository::RecordsRepository;

    use super::ViewRepoDialogue;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn repo_with_records() -> (TempDir, RecordsFileRepository) {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let mut repo = RecordsFileRepository::new(path, "pass".to_string());

        let rec1 = Record::new(vec![
            (RECORD_NAME_FIELD.to_string(), "Mail".to_string()),
            (RECORD_PASSWD_FIELD.to_string(), "pw1".to_string()),
            (
                RECORD_LOGIN_FIELD.to_string(),
                "user@example.com".to_string(),
            ),
        ]);
        let rec2 = Record::new(vec![
            (RECORD_NAME_FIELD.to_string(), "Github".to_string()),
            (RECORD_PASSWD_FIELD.to_string(), "pw2".to_string()),
            (RECORD_LOGIN_FIELD.to_string(), "octocat".to_string()),
        ]);

        repo.add_record(rec1).expect("add rec1");
        repo.add_record(rec2).expect("add rec2");
        repo.save().expect("save repo");
        (tmp, repo)
    }

    #[test]
    fn test_navigation_wraps_up() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));
        let res = dialogue.handle_key(key(KeyCode::Up));

        assert!(matches!(res, DialogueResult::NoOp));
        assert_eq!(dialogue.list_state.selected(), Some(3));
    }

    #[test]
    fn test_navigation_wraps_down() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(3));
        let res = dialogue.handle_key(key(KeyCode::Down));

        assert!(matches!(res, DialogueResult::NoOp));
        assert_eq!(dialogue.list_state.selected(), Some(0));
    }

    #[test]
    fn test_search_starts_on_slash() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));
        let res = dialogue.handle_key(key(KeyCode::Char('/')));

        assert!(matches!(res, DialogueResult::NoOp));
        assert!(dialogue.is_searching);
        assert_eq!(dialogue.search_query, "");
    }

    #[test]
    fn test_search_typing_and_backspace() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(1));
        let _ = dialogue.handle_key(key(KeyCode::Char('/')));

        let res1 = dialogue.handle_key(key(KeyCode::Char('m')));
        assert!(matches!(res1, DialogueResult::NoOp));
        assert_eq!(dialogue.search_query, "m");
        assert_eq!(dialogue.list_state.selected(), Some(0));

        let res2 = dialogue.handle_key(key(KeyCode::Backspace));
        assert!(matches!(res2, DialogueResult::NoOp));
        assert_eq!(dialogue.search_query, "");
    }

    #[test]
    fn test_search_esc_clears_and_stops_search() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(1));
        let _ = dialogue.handle_key(key(KeyCode::Char('/')));
        let _ = dialogue.handle_key(key(KeyCode::Char('x')));
        let res = dialogue.handle_key(key(KeyCode::Esc));

        assert!(matches!(res, DialogueResult::NoOp));
        assert!(!dialogue.is_searching);
        assert_eq!(dialogue.search_query, "");
        assert_eq!(dialogue.list_state.selected(), Some(0));
    }

    #[test]
    fn test_filter_matches_name_case_insensitive() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));
        dialogue.is_searching = true;
        dialogue.search_query = "gIt".to_string();

        let rows = dialogue.get_filtered_records();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "Github");
    }

    #[test]
    fn test_filter_matches_login_case_insensitive() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));
        dialogue.is_searching = true;
        dialogue.search_query = "EXAMPLE".to_string();

        let rows = dialogue.get_filtered_records();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "Mail");
    }

    #[test]
    fn test_enter_record_changes_to_view_record() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));

        let res = dialogue.handle_key(key(KeyCode::Enter));
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_char_a_starts_add_record_input() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));

        let res = dialogue.handle_key(key(KeyCode::Char('a')));
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
    fn test_char_c_closes_repository() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(0));
        let res = dialogue.handle_key(key(KeyCode::Char('c')));
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_enter_add_row_starts_add_record_input() {
        let (_tmp, repo) = repo_with_records();
        let mut dialogue = ViewRepoDialogue::new(repo, Some(2));
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
}
