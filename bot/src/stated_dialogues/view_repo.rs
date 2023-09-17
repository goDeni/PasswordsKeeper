use std::collections::HashSet;

use super::{
    add_record::AddRecordDialog, common::RECORD_NAME_FIELD, view_record::ViewRecordDialog,
    ButtonPayload, CtxResult, DialContext, Message, MessageId, Select,
};
use anyhow::Result;
use sec_store::repository::RecordsRepository;

const CLOSE_REPO: &'static str = "CLOSE_REPO";
const ADD_RECORD: &'static str = "ADD_RECORD";

pub struct ViewRepoDialog<T> {
    repo: T,
    sent_msg_ids: HashSet<MessageId>,
}

impl<T> ViewRepoDialog<T> {
    pub fn new(repo: T) -> Self {
        ViewRepoDialog {
            repo,
            sent_msg_ids: HashSet::new(),
        }
    }
}

impl<T> DialContext for ViewRepoDialog<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        let records_buttons = self
            .repo
            .get_records()
            .into_iter()
            .map(|record| {
                (
                    record.id.clone(),
                    record
                        .get_field_value(&RECORD_NAME_FIELD.into())
                        .unwrap_or("-".to_string()),
                )
            })
            .map(|(id, name)| vec![(id.into(), name)])
            .collect::<Vec<Vec<(ButtonPayload, String)>>>();

        let records_count = records_buttons.len();
        let mut buttons = records_buttons;
        buttons.extend(vec![
            vec![(ADD_RECORD.into(), "Добавить запись 🗒".into())],
            vec![(CLOSE_REPO.into(), "Закрыть репозиторий 🚪".into())],
        ]);
        Ok(vec![CtxResult::Buttons(
            format!("Количество записей: {}", records_count).into(),
            buttons,
        )])
    }

    fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match select.data() {
            Some(CLOSE_REPO) => CtxResult::CloseCtx,
            Some(ADD_RECORD) => {
                CtxResult::NewCtx(Box::new(AddRecordDialog::new(self.repo.clone())))
            }
            Some(record_id) => match self.repo.get(&record_id.to_string()) {
                Some(_) => CtxResult::NewCtx(Box::new(ViewRecordDialog::new(
                    self.repo.clone(),
                    record_id.to_string(),
                ))),
                None => CtxResult::Nothing,
            },
            _ => CtxResult::Nothing,
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
