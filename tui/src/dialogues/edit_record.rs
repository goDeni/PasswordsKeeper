use ratatui::symbols::border;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use sec_store::repository::file::RecordsFileRepository;
use sec_store::repository::RecordsRepository;

use crate::dialogues::{Dialogue, DialogueResult};
use crate::fields::{
    RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD,
};
use crate::runtime::block_on;

type RecordId = String;

#[derive(Debug)]
pub struct EditRecordDialogue {
    repo: RecordsFileRepository,
    record_id: RecordId,
    list_state: ListState,
    editing_field: Option<String>,
}

impl EditRecordDialogue {
    pub fn new(repo: RecordsFileRepository, record_id: RecordId, selected: Option<usize>) -> Self {
        let mut state = ListState::default();
        state.select(selected);
        Self {
            repo,
            record_id,
            list_state: state,
            editing_field: None,
        }
    }
}

impl Dialogue for EditRecordDialogue {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Edit record ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().yellow());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(ref field) = self.editing_field {
            let r = self.repo.clone();
            if let Ok(Some(rec)) = block_on(r.get(&self.record_id)) {
                let val = rec.get_field_value(field).unwrap_or_default();
                let label = match field.as_str() {
                    RECORD_NAME_FIELD => "Name",
                    RECORD_LOGIN_FIELD => "Login",
                    RECORD_DESCR_FIELD => "Description",
                    RECORD_PASSWD_FIELD => "Password",
                    _ => field,
                };
                let text = format!("Editing {label}:\n{val}");
                frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
            }
        } else {
            let r = self.repo.clone();
            let rec = match block_on(r.get(&self.record_id)) {
                Ok(Some(x)) => x,
                _ => return,
            };
            let mut items = vec![ListItem::new("Name"), ListItem::new("Password")];
            if rec.get_field_value(RECORD_LOGIN_FIELD).is_some() {
                items.push(ListItem::new("Login"));
            }
            if rec.get_field_value(RECORD_DESCR_FIELD).is_some() {
                items.push(ListItem::new("Description"));
            }
            let list = List::new(items.clone())
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">> ");
            let sel = self.list_state.selected().unwrap_or(0);
            let mut state = ListState::default();
            state.select(Some(sel.min(items.len().saturating_sub(1))));
            frame.render_stateful_widget(list, inner, &mut state);
        }

        let instructions = Line::from(vec![
            Span::styled("Enter", Style::new().cyan()),
            Span::raw(" select "),
            Span::styled("Esc", Style::new().cyan()),
            Span::raw(" cancel "),
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

    fn handle_key(&mut self, k: crossterm::event::KeyEvent) -> DialogueResult {
        use crossterm::event::KeyCode;

        if self.editing_field.is_some() {
            // In edit value mode, handled by input
            return DialogueResult::NoOp;
        }

        let r = self.repo.clone();
        let rec = match block_on(r.get(&self.record_id)) {
            Ok(Some(x)) => x,
            _ => return DialogueResult::NoOp,
        };
        let mut fields = vec![RECORD_NAME_FIELD, RECORD_PASSWD_FIELD];
        if rec.get_field_value(RECORD_LOGIN_FIELD).is_some() {
            fields.push(RECORD_LOGIN_FIELD);
        }
        if rec.get_field_value(RECORD_DESCR_FIELD).is_some() {
            fields.push(RECORD_DESCR_FIELD);
        }
        let n = fields.len();
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
            KeyCode::Enter => {
                let f = fields[sel].to_string();
                self.editing_field = Some(f.clone());
                DialogueResult::StartInput {
                    prompt: "Enter new value".to_string(),
                    password: f == RECORD_PASSWD_FIELD,
                }
            }
            KeyCode::Esc => DialogueResult::ChangeScreen(Box::new(
                crate::dialogues::view_record::ViewRecordDialogue::new(
                    self.repo.clone(),
                    self.record_id.clone(),
                    false,
                ),
            )),
            _ => DialogueResult::NoOp,
        }
    }

    fn on_input_submit(&mut self, value: String) -> DialogueResult {
        if let Some(field) = self.editing_field.clone() {
            let mut r = self.repo.clone();
            let rid = self.record_id.clone();
            if let Ok(Some(mut rec)) = block_on(r.get(&rid)) {
                if rec.update_field(field.clone(), value).is_ok()
                    && block_on(r.update(rec)).is_ok()
                    && block_on(r.save()).is_ok()
                {
                    return DialogueResult::ChangeScreen(Box::new(
                        crate::dialogues::view_record::ViewRecordDialogue::new(r, rid, false),
                    ));
                }
            }
            self.editing_field = Some(field.clone());
            DialogueResult::Error("Update failed".to_string())
        } else {
            DialogueResult::NoOp
        }
    }

    fn on_input_cancel(&mut self) -> DialogueResult {
        self.editing_field = None;
        DialogueResult::NoOp
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tempfile::TempDir;

    use crate::dialogues::{Dialogue, DialogueResult};
    use crate::fields::{
        RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD,
    };
    use crate::runtime::block_on;
    use crate::test_helpers::test_password;
    use sec_store::record::Record;
    use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
    use sec_store::repository::{OpenRepository, RecordsRepository};

    use super::EditRecordDialogue;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_repo_with_full_record() -> (TempDir, RecordsFileRepository, String, String) {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let repo_password = test_password();
        let mut repo = RecordsFileRepository::new(path, repo_password.clone());
        let rec = Record::new(vec![
            (RECORD_NAME_FIELD.to_string(), "Mail".to_string()),
            (RECORD_PASSWD_FIELD.to_string(), "pw".to_string()),
            (RECORD_LOGIN_FIELD.to_string(), "user".to_string()),
            (RECORD_DESCR_FIELD.to_string(), "desc".to_string()),
        ]);
        let id = rec.id.clone();
        block_on(repo.add_record(rec)).expect("add");
        block_on(repo.save()).expect("save");
        (tmp, repo, id, repo_password)
    }

    #[test]
    fn test_navigation_wraps_up() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id, Some(0));

        let res = dialogue.handle_key(key(KeyCode::Up));
        assert!(matches!(res, DialogueResult::NoOp));
        assert_eq!(dialogue.list_state.selected(), Some(3));
    }

    #[test]
    fn test_navigation_wraps_down() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id, Some(3));

        let res = dialogue.handle_key(key(KeyCode::Down));
        assert!(matches!(res, DialogueResult::NoOp));
        assert_eq!(dialogue.list_state.selected(), Some(0));
    }

    #[test]
    fn test_enter_name_field_starts_plain_input() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id, Some(0));

        let res = dialogue.handle_key(key(KeyCode::Enter));
        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Enter new value");
                assert!(!password);
            }
            _ => panic!("expected StartInput"),
        }
        assert_eq!(dialogue.editing_field.as_deref(), Some(RECORD_NAME_FIELD));
    }

    #[test]
    fn test_enter_password_field_starts_password_input() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id, Some(1));

        let res = dialogue.handle_key(key(KeyCode::Enter));
        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Enter new value");
                assert!(password);
            }
            _ => panic!("expected StartInput"),
        }
        assert_eq!(dialogue.editing_field.as_deref(), Some(RECORD_PASSWD_FIELD));
    }

    #[test]
    fn test_escape_returns_to_view_record() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id, Some(0));

        let res = dialogue.handle_key(key(KeyCode::Esc));
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_input_submit_updates_record() {
        let (tmp, repo, id, repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id.clone(), Some(0));
        let _ = dialogue.handle_key(key(KeyCode::Enter));

        let res = dialogue.on_input_submit("New Mail".to_string());
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));

        let mut repo_after = OpenRecordsFileRepository(tmp.path().join("repo")).open(repo_password);
        let mut repo_after = block_on(repo_after).expect("open repo");
        let rec = block_on(repo_after.get(&id))
            .expect("get")
            .expect("record must exist");
        assert_eq!(
            rec.get_field_value(RECORD_NAME_FIELD).as_deref(),
            Some("New Mail")
        );
    }

    #[test]
    fn test_input_cancel_exits_edit_mode() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_full_record();
        let mut dialogue = EditRecordDialogue::new(repo, id, Some(0));
        let _ = dialogue.handle_key(key(KeyCode::Enter));
        assert!(dialogue.editing_field.is_some());

        let res = dialogue.on_input_cancel();
        assert!(matches!(res, DialogueResult::NoOp));
        assert!(dialogue.editing_field.is_none());
    }

    #[test]
    fn test_missing_record_returns_noop() {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let mut repo = RecordsFileRepository::new(path, test_password());
        block_on(repo.save()).expect("save");

        let mut dialogue = EditRecordDialogue::new(repo, "missing-id".to_string(), Some(0));
        let res = dialogue.handle_key(key(KeyCode::Enter));
        assert!(matches!(res, DialogueResult::NoOp));
    }
}
