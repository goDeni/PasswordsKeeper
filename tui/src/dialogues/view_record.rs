use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

use ratatui::symbols::border;
use sec_store::repository::file::RecordsFileRepository;
use sec_store::repository::RecordsRepository;

use crate::dialogues::{Dialogue, DialogueResult};
use crate::fields::{
    RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD,
};
use crate::runtime::block_on;

type RecordId = String;

#[derive(Debug)]
pub struct ViewRecordDialogue {
    repo: RecordsFileRepository,
    record_id: RecordId,
    confirm_delete: bool,
    password_visible: bool,
}

impl ViewRecordDialogue {
    pub fn new(repo: RecordsFileRepository, record_id: RecordId, confirm_delete: bool) -> Self {
        Self {
            repo,
            record_id,
            confirm_delete,
            password_visible: false,
        }
    }
}

impl Dialogue for ViewRecordDialogue {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Record ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().yellow());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let repo = self.repo.clone();
        let rec = match block_on(repo.get(&self.record_id)) {
            Ok(Some(r)) => r,
            _ => {
                let p = Paragraph::new("Record not found.");
                frame.render_widget(p, inner);
                return;
            }
        };

        let password_display = if let Some(pwd) = rec.get_field_value(RECORD_PASSWD_FIELD) {
            if self.password_visible {
                pwd
            } else {
                "*".repeat(pwd.len())
            }
        } else {
            String::new()
        };

        let mut lines = vec![
            format!(
                "Name: {}",
                rec.get_field_value(RECORD_NAME_FIELD).unwrap_or_default()
            ),
            format!("Password: {}", password_display),
        ];
        if let Some(l) = rec.get_field_value(RECORD_LOGIN_FIELD) {
            lines.push(format!("Login: {l}"));
        }
        if let Some(d) = rec.get_field_value(RECORD_DESCR_FIELD) {
            lines.push(format!("Description: {d}"));
        }
        if self.confirm_delete {
            lines.push(String::new());
            lines.push("Do you really want to remove this record? (Y/N)".to_string());
        }
        let text = lines.join("\n");
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);

        let instructions = if self.confirm_delete {
            Line::from(vec![
                Span::styled("Y", Style::new().cyan()),
                Span::raw(" yes, remove "),
                Span::styled("N", Style::new().cyan()),
                Span::raw(" / "),
                Span::styled("Esc", Style::new().cyan()),
                Span::raw(" no, cancel "),
                Span::styled("q", Style::new().cyan()),
                Span::raw(" quit"),
            ])
        } else {
            Line::from(vec![
                Span::styled("e", Style::new().cyan()),
                Span::raw(" edit "),
                Span::styled("c", Style::new().cyan()),
                Span::raw(" copy "),
                Span::styled("Ctrl+v", Style::new().cyan()),
                Span::raw(" toggle password "),
                Span::styled("d", Style::new().cyan()),
                Span::raw(" delete "),
                Span::styled("b", Style::new().cyan()),
                Span::raw(" back "),
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
        let rid = self.record_id.clone();

        // Handle Ctrl+V to toggle password visibility
        if k.code == KeyCode::Char('v')
            && k.modifiers.contains(KeyModifiers::CONTROL)
            && !self.confirm_delete
        {
            self.password_visible = !self.password_visible;
            return DialogueResult::NoOp;
        }

        match k.code {
            KeyCode::Char('y') | KeyCode::Char('Y') if self.confirm_delete => {
                let mut r = self.repo.clone();
                if block_on(r.delete(&rid)).is_ok() && block_on(r.save()).is_ok() {
                    DialogueResult::ChangeScreen(Box::new(
                        crate::dialogues::view_repo::ViewRepoDialogue::new(r, Some(0)),
                    ))
                } else {
                    DialogueResult::Error("Delete failed".to_string())
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc if self.confirm_delete => {
                self.confirm_delete = false;
                DialogueResult::NoOp
            }
            KeyCode::Char('c') if !self.confirm_delete => {
                let repo = self.repo.clone();
                if let Ok(Some(rec)) = block_on(repo.get(&rid)) {
                    if let Some(password) = rec.get_field_value(RECORD_PASSWD_FIELD) {
                        match std::process::Command::new("wl-copy")
                            .stdin(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()
                        {
                            Ok(mut child) => {
                                let result = if let Some(mut stdin) = child.stdin.take() {
                                    use std::io::Write;
                                    match stdin.write_all(password.as_bytes()) {
                                        Ok(_) => {
                                            // Close stdin to signal EOF
                                            drop(stdin);
                                            match child.wait() {
                                                Ok(status) => {
                                                    if status.success() {
                                                        DialogueResult::Success("Password copied to clipboard".to_string())
                                                    } else {
                                                        DialogueResult::Error(format!(
                                                            "wl-copy failed with exit code: {}",
                                                            status.code().unwrap_or(-1)
                                                        ))
                                                    }
                                                }
                                                Err(err) => DialogueResult::Error(format!(
                                                    "Failed to wait for wl-copy: {}",
                                                    err
                                                )),
                                            }
                                        }
                                        Err(err) => {
                                            let _ = child.kill();
                                            DialogueResult::Error(format!(
                                                "Failed to write to wl-copy: {}",
                                                err
                                            ))
                                        }
                                    }
                                } else {
                                    let _ = child.kill();
                                    DialogueResult::Error("Failed to get stdin for wl-copy".to_string())
                                };
                                result
                            }
                            Err(err) => DialogueResult::Error(format!(
                                "Failed to execute wl-copy: {}. Make sure wl-clipboard is installed.",
                                err
                            )),
                        }
                    } else {
                        DialogueResult::Error("Password not found".to_string())
                    }
                } else {
                    DialogueResult::Error("Record not found".to_string())
                }
            }
            KeyCode::Char('e') if !self.confirm_delete => DialogueResult::ChangeScreen(Box::new(
                crate::dialogues::edit_record::EditRecordDialogue::new(
                    self.repo.clone(),
                    rid,
                    Some(0),
                ),
            )),
            KeyCode::Char('d') if !self.confirm_delete => {
                self.confirm_delete = true;
                DialogueResult::NoOp
            }
            KeyCode::Char('b') if !self.confirm_delete => DialogueResult::ChangeScreen(Box::new(
                crate::dialogues::view_repo::ViewRepoDialogue::new(self.repo.clone(), Some(0)),
            )),
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
    use crate::fields::{RECORD_NAME_FIELD, RECORD_PASSWD_FIELD};
    use crate::runtime::block_on;
    use crate::test_helpers::test_password;
    use sec_store::record::Record;
    use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
    use sec_store::repository::{OpenRepository, RecordsRepository};

    use super::ViewRecordDialogue;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_v() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)
    }

    fn make_repo_with_password_record() -> (TempDir, RecordsFileRepository, String, String) {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let repo_password = test_password();
        let mut repo = RecordsFileRepository::new(path, repo_password.clone());
        let rec = Record::new(vec![
            (RECORD_NAME_FIELD.to_string(), "Mail".to_string()),
            (RECORD_PASSWD_FIELD.to_string(), "pw".to_string()),
        ]);
        let id = rec.id.clone();
        block_on(repo.add_record(rec)).expect("add");
        block_on(repo.save()).expect("save");
        (tmp, repo, id, repo_password)
    }

    fn make_repo_without_password_record() -> (TempDir, RecordsFileRepository, String) {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let mut repo = RecordsFileRepository::new(path, test_password());
        let rec = Record::new(vec![(RECORD_NAME_FIELD.to_string(), "Mail".to_string())]);
        let id = rec.id.clone();
        block_on(repo.add_record(rec)).expect("add");
        block_on(repo.save()).expect("save");
        (tmp, repo, id)
    }

    #[test]
    fn test_ctrl_v_toggles_password_visibility() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, false);

        assert!(!dialogue.password_visible);
        let res = dialogue.handle_key(ctrl_v());
        assert!(matches!(res, DialogueResult::NoOp));
        assert!(dialogue.password_visible);
    }

    #[test]
    fn test_delete_key_enters_confirmation_mode() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, false);

        let res = dialogue.handle_key(key(KeyCode::Char('d')));
        assert!(matches!(res, DialogueResult::NoOp));
        assert!(dialogue.confirm_delete);
    }

    #[test]
    fn test_esc_in_confirmation_cancels_delete() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, true);

        let res = dialogue.handle_key(key(KeyCode::Esc));
        assert!(matches!(res, DialogueResult::NoOp));
        assert!(!dialogue.confirm_delete);
    }

    #[test]
    fn test_n_in_confirmation_cancels_delete() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, true);

        let res = dialogue.handle_key(key(KeyCode::Char('n')));
        assert!(matches!(res, DialogueResult::NoOp));
        assert!(!dialogue.confirm_delete);
    }

    #[test]
    fn test_y_in_confirmation_deletes_record_and_changes_screen() {
        let (tmp, repo, id, repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id.clone(), true);

        let res = dialogue.handle_key(key(KeyCode::Char('y')));
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));

        let mut repo_after = OpenRecordsFileRepository(tmp.path().join("repo")).open(repo_password);
        let mut repo_after = block_on(repo_after).expect("open repo");
        let found = block_on(repo_after.get(&id)).expect("get");
        assert!(found.is_none());
    }

    #[test]
    fn test_back_returns_to_repo() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, false);
        let res = dialogue.handle_key(key(KeyCode::Char('b')));
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_edit_returns_edit_screen() {
        let (_tmp, repo, id, _repo_password) = make_repo_with_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, false);
        let res = dialogue.handle_key(key(KeyCode::Char('e')));
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_copy_with_missing_record_returns_error() {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let mut repo = RecordsFileRepository::new(path, test_password());
        block_on(repo.save()).expect("save");

        let mut dialogue = ViewRecordDialogue::new(repo, "missing-id".to_string(), false);
        let res = dialogue.handle_key(key(KeyCode::Char('c')));
        match res {
            DialogueResult::Error(msg) => assert_eq!(msg, "Record not found"),
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn test_copy_with_missing_password_returns_error() {
        let (_tmp, repo, id) = make_repo_without_password_record();
        let mut dialogue = ViewRecordDialogue::new(repo, id, false);

        let res = dialogue.handle_key(key(KeyCode::Char('c')));
        match res {
            DialogueResult::Error(msg) => assert_eq!(msg, "Password not found"),
            _ => panic!("expected error"),
        }
    }
}
