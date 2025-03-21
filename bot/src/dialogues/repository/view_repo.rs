use async_trait::async_trait;
use std::collections::HashSet;

use anyhow::Result;
use sec_store::repository::RecordsRepository;
use stated_dialogues::dialogues::{
    ButtonPayload, CtxResult, DialContext, Message, MessageId, OutgoingDocument, Select,
};

use super::super::commands::BACKUP_COMMAND;
use crate::dialogues::commands::default_commands_handler;

use super::records::{
    add_record::AddRecordDialog, fields::RECORD_NAME_FIELD, view_record::ViewRecordDialog,
};

const CLOSE_REPO: &str = "CLOSE_REPO";
const ADD_RECORD: &str = "ADD_RECORD";

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

#[async_trait]
impl<T> DialContext for ViewRepoDialog<T>
where
    T: RecordsRepository,
{
    async fn init(&mut self) -> Result<Vec<CtxResult>> {
        let mut records_buttons = self
            .repo
            .get_records()?
            .into_iter()
            .map(|record| {
                (
                    record.id.clone(),
                    record
                        .get_field_value(RECORD_NAME_FIELD)
                        .unwrap_or("-".to_string()),
                )
            })
            .map(|(id, name)| vec![(id.into(), name)])
            .collect::<Vec<Vec<(ButtonPayload, String)>>>();
        records_buttons.sort_by(|a, b| {
            a.is_empty()
                .then(|| a.len().cmp(&b.len()))
                .unwrap_or_else(|| a[0].1.cmp(&b[0].1))
        });

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

    async fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    async fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match select.data() {
            Some(CLOSE_REPO) => CtxResult::CloseCtx,
            Some(ADD_RECORD) => {
                CtxResult::NewCtx(Box::new(AddRecordDialog::new(self.repo.clone())))
            }
            Some(record_id) => match self.repo.get(&record_id.to_string())? {
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

    async fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![message.id])])
    }

    async fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        Ok(match (command.text(), &command.user_id) {
            (Some(BACKUP_COMMAND), Some(user_id)) => {
                vec![
                    CtxResult::Document(OutgoingDocument::new(
                        self.repo.dump()?,
                        format!("user_{}.json", user_id),
                    )),
                    CtxResult::RemoveMessages(vec![command.id]),
                ]
            }
            _ => default_commands_handler(command),
        })
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
