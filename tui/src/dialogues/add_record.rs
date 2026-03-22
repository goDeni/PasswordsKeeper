use ratatui::symbols::border;
use ratatui::{layout::Rect, widgets::Block, Frame};
use sec_store::repository::RecordsRepository;

use crate::dialogues::{Dialogue, DialogueResult};
use crate::record_fields::RecordFields;
use crate::repo::{self, RepositoryFactory};

#[derive(Debug)]
pub struct AddRecordDialogue<F, R> {
    factory: F,
    repo: R,
    record_fields: RecordFields,
}

impl<F, R> AddRecordDialogue<F, R> {
    pub fn new(factory: F, repo: R) -> Self {
        Self {
            factory,
            repo,
            record_fields: RecordFields::new(),
        }
    }
}

impl<F, R> Dialogue<F, R> for AddRecordDialogue<F, R>
where
    F: RepositoryFactory<R>,
    R: RecordsRepository,
{
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Add record ")
            .border_set(border::ROUNDED);
        frame.render_widget(block, area);
    }

    fn handle_key(&mut self, _k: crossterm::event::KeyEvent) -> DialogueResult<F, R> {
        DialogueResult::NoOp
    }

    fn on_input_submit(&mut self, value: String) -> DialogueResult<F, R> {
        use crate::fields::{
            RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD,
        };
        use crate::record_fields::AddRecordStep;
        use sec_store::record::Record;
        let step = self.record_fields.get_current_step();
        match step {
            AddRecordStep::Password => {
                if value.is_empty() {
                    return DialogueResult::Error("Password cannot be empty".to_string());
                }
                self.record_fields.password = value;
                DialogueResult::StartInput {
                    prompt: "Enter name".to_string(),
                    password: false,
                }
            }
            AddRecordStep::Name => {
                if value.is_empty() {
                    return DialogueResult::Error("Name cannot be empty".to_string());
                }
                self.record_fields.name = value;
                DialogueResult::StartInput {
                    prompt: "Enter login (or leave empty to skip)".to_string(),
                    password: false,
                }
            }
            AddRecordStep::Login => {
                self.record_fields.login = Some(value);
                DialogueResult::StartInput {
                    prompt: "Enter description (or leave empty to skip)".to_string(),
                    password: false,
                }
            }
            AddRecordStep::Description => {
                self.record_fields.description = if value.is_empty() { None } else { Some(value) };
                let record_fields = &self.record_fields;
                let mut fields = vec![
                    (RECORD_NAME_FIELD.to_string(), record_fields.name.clone()),
                    (
                        RECORD_PASSWD_FIELD.to_string(),
                        record_fields.password.clone(),
                    ),
                ];
                if let Some(l) = record_fields
                    .login
                    .as_ref()
                    .filter(|login| !login.is_empty())
                {
                    fields.push((RECORD_LOGIN_FIELD.to_string(), l.clone()));
                }
                // Always add description field, even if empty
                let desc_value = record_fields.description.as_deref().unwrap_or("");
                fields.push((RECORD_DESCR_FIELD.to_string(), desc_value.to_string()));
                let record = Record::new(fields);
                if let Err(e) = repo::add_record(&mut self.repo, record) {
                    return DialogueResult::Error(e.to_string());
                }
                if let Err(e) = repo::save(&mut self.repo) {
                    return DialogueResult::Error(e.to_string());
                }
                let repo = self.repo.clone();
                DialogueResult::ChangeScreen(Box::new(
                    crate::dialogues::view_repo::ViewRepoDialogue::new(
                        self.factory.clone(),
                        repo,
                        Some(0),
                    ),
                ))
            }
            AddRecordStep::Complete => DialogueResult::NoOp,
        }
    }

    fn on_input_cancel(&mut self) -> DialogueResult<F, R> {
        let repo = self.repo.clone();
        DialogueResult::ChangeScreen(Box::new(
            crate::dialogues::view_repo::ViewRepoDialogue::new(self.factory.clone(), repo, Some(0)),
        ))
    }

    fn on_exit(&mut self) {
        let _ = repo::close_connection(&self.repo);
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::dialogues::{Dialogue, DialogueResult};
    use crate::fields::RECORD_LOGIN_FIELD;
    use crate::repo::FileRepositoryFactory;
    use crate::runtime::block_on;
    use crate::test_helpers::test_password;
    use sec_store::repository::file::{OpenRecordsFileRepository, RecordsFileRepository};
    use sec_store::repository::{OpenRepository, RecordsRepository};

    use super::AddRecordDialogue;

    fn make_repo() -> (
        TempDir,
        FileRepositoryFactory,
        RecordsFileRepository,
        String,
    ) {
        let tmp = TempDir::new().expect("temp dir");
        let path = tmp.path().join("repo");
        let repo_password = test_password();
        let mut repo = RecordsFileRepository::new(path, repo_password.clone());
        crate::runtime::block_on(repo.save()).expect("save repo");
        let factory = FileRepositoryFactory::new(tmp.path().join("repo"));
        (tmp, factory, repo, repo_password)
    }

    #[test]
    fn test_empty_password_returns_error() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);

        let res = dialogue.on_input_submit(String::new());
        match res {
            DialogueResult::Error(msg) => assert_eq!(msg, "Password cannot be empty"),
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn test_password_step_moves_to_name() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);

        let res = dialogue.on_input_submit("pw".to_string());
        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Enter name");
                assert!(!password);
            }
            _ => panic!("expected StartInput"),
        }
    }

    #[test]
    fn test_empty_name_returns_error() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let _ = dialogue.on_input_submit("pw".to_string());

        let res = dialogue.on_input_submit(String::new());
        match res {
            DialogueResult::Error(msg) => assert_eq!(msg, "Name cannot be empty"),
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn test_name_step_moves_to_login() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let _ = dialogue.on_input_submit("pw".to_string());

        let res = dialogue.on_input_submit("mail".to_string());
        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Enter login (or leave empty to skip)");
                assert!(!password);
            }
            _ => panic!("expected StartInput"),
        }
    }

    #[test]
    fn test_login_step_moves_to_description() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let _ = dialogue.on_input_submit("pw".to_string());
        let _ = dialogue.on_input_submit("mail".to_string());

        let res = dialogue.on_input_submit("user".to_string());
        match res {
            DialogueResult::StartInput { prompt, password } => {
                assert_eq!(prompt, "Enter description (or leave empty to skip)");
                assert!(!password);
            }
            _ => panic!("expected StartInput"),
        }
    }

    #[test]
    fn test_complete_flow_adds_record_and_changes_screen() {
        let (tmp, factory, repo, repo_password) = make_repo();
        let path = tmp.path().join("repo");
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let _ = dialogue.on_input_submit("pw".to_string());
        let _ = dialogue.on_input_submit("mail".to_string());
        let _ = dialogue.on_input_submit("user".to_string());

        let res = dialogue.on_input_submit("desc".to_string());
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));

        let opened = OpenRecordsFileRepository(path).open(repo_password);
        let opened = block_on(opened).expect("open saved repo");
        assert_eq!(block_on(opened.get_records()).expect("records").len(), 1);
    }

    #[test]
    fn test_complete_flow_with_empty_optional_fields_still_succeeds() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let _ = dialogue.on_input_submit("pw".to_string());
        let _ = dialogue.on_input_submit("mail".to_string());
        let _ = dialogue.on_input_submit(String::new());

        let res = dialogue.on_input_submit(String::new());
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_cancel_changes_to_view_repo() {
        let (_tmp, factory, repo, _repo_password) = make_repo();
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let res = dialogue.on_input_cancel();
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));
    }

    #[test]
    fn test_empty_login_is_not_persisted() {
        let (tmp, factory, repo, repo_password) = make_repo();
        let path = tmp.path().join("repo");
        let mut dialogue = AddRecordDialogue::new(factory, repo);
        let _ = dialogue.on_input_submit("record-password".to_string());
        let _ = dialogue.on_input_submit("mail".to_string());
        let _ = dialogue.on_input_submit(String::new());

        let res = dialogue.on_input_submit(String::new());
        assert!(matches!(res, DialogueResult::ChangeScreen(_)));

        let opened = OpenRecordsFileRepository(path).open(repo_password);
        let opened = block_on(opened).expect("open saved repo");
        let record = block_on(opened.get_records())
            .expect("records")
            .into_iter()
            .next()
            .expect("record must exist");
        assert!(record.get_field_value(RECORD_LOGIN_FIELD).is_none());
    }
}
