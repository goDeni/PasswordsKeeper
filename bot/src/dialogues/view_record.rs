use std::collections::HashSet;

use sec_store::{record::RecordId, repository::RecordsRepository};

use super::{common::record_as_message, edit_record::EditRecordDialog, view_repo::ViewRepoDialog};
use crate::stated_dialogues::{CtxResult, DialContext, Message, MessageId, Select};
use anyhow::Result;

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

impl<T> DialContext for ViewRecordDialog<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match self.repo.get(&self.record_id) {
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

    fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
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

    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![message.id])])
    }

    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![command.id])])
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
}
