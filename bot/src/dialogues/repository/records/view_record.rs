use crate::dialogues::commands::default_commands_handler;
use std::collections::HashSet;

use sec_store::{record::RecordId, repository::RecordsRepository};

use super::{edit_record::EditRecordDialog, fields::record_as_message};
use crate::dialogues::repository::view_repo::ViewRepoDialog;
use anyhow::Result;
use async_trait::async_trait;
use stated_dialogues::dialogues::{CtxResult, DialContext, Message, MessageId, Select};

pub struct ViewRecordDialog<T> {
    repo: T,
    record_id: RecordId,
    sent_msg_ids: HashSet<MessageId>,
}

impl<T> ViewRecordDialog<T> {
    pub fn new(repo: T, record_id: RecordId) -> Self {
        ViewRecordDialog {
            record_id,
            repo,
            sent_msg_ids: HashSet::new(),
        }
    }
}

const EDIT_RECORD: &str = "EDIT_RECORD";
const REMOVE_RECORD: &str = "REMOVE_RECORD";
const CLOSE_VIEW: &str = "CLOSE_VIEW";

#[async_trait]
impl<T> DialContext for ViewRecordDialog<T>
where
    T: RecordsRepository,
{
    async fn init(&mut self) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match self.repo.get(&self.record_id)? {
            Some(record) => CtxResult::Buttons(
                record_as_message(record),
                vec![
                    vec![(EDIT_RECORD.into(), "✏️".into())],
                    vec![(REMOVE_RECORD.into(), "❌".into())],
                    vec![(CLOSE_VIEW.into(), "⬅️ Закрыть".into())],
                ],
            ),
            None => CtxResult::NewCtx(Box::new(ViewRepoDialog::new(self.repo.clone()))),
        };

        Ok(vec![result])
    }

    async fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    async fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        let result = match select.data() {
            Some(EDIT_RECORD) => CtxResult::NewCtx(Box::new(EditRecordDialog::new(
                self.record_id.clone(),
                self.repo.clone(),
            ))),
            Some(REMOVE_RECORD) => {
                self.repo.delete(&self.record_id)?;
                self.repo.save()?;
                CtxResult::NewCtx(Box::new(ViewRepoDialog::new(self.repo.clone())))
            }
            Some(CLOSE_VIEW) => CtxResult::NewCtx(Box::new(ViewRepoDialog::new(self.repo.clone()))),
            other => {
                log::warn!("Unexpected select called {:?}", other);
                CtxResult::CloseCtx
            }
        };

        Ok(vec![result])
    }

    async fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![message.id])])
    }

    async fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        Ok(default_commands_handler(command))
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
    fn file_expected(&self) -> bool {
        false
    }
}
