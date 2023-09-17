use std::collections::{HashMap, HashSet};

use sec_store::{
    record::{Record, RecordId},
    repository::RecordsRepository,
};

use super::{
    common::{RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD},
    view_repo::ViewRepoDialog,
    CtxResult, DialContext, Message, MessageFormat, MessageId, OutgoingMessage, Select,
};
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

fn record_as_message(record: &Record) -> String {
    let fields: HashMap<String, String> = HashMap::from_iter(
        record
            .get_fields()
            .into_iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );

    let mut lines: Vec<String> = vec![format!(
        "Название: <code>{}</code>",
        fields[RECORD_NAME_FIELD]
    )];
    if let Some(login) = fields.get(RECORD_LOGIN_FIELD) {
        lines.push(format!("Логин: <code>{}</code>", login));
    }
    if let Some(descr) = fields.get(RECORD_DESCR_FIELD) {
        lines.push(format!("Описание: <code>{}</code>", descr));
    }

    lines.push(format!(
        "Пароль: <code>{}</code>",
        fields[RECORD_PASSWD_FIELD]
    ));
    lines.join("\n")
}

const EDIT_RECORD: &'static str = "EDIT_RECORD";
const REMOVE_RECORD: &'static str = "REMOVE_RECORD";
const CLOSE_VIEW: &'static str = "CLOSE_VIEW";

impl<T> DialContext for ViewRecordDialog<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match self.repo.get(&self.record_id) {
            Some(record) => CtxResult::Buttons(
                OutgoingMessage::new(record_as_message(&record), MessageFormat::Html),
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
            self.sent_msg_ids
                .clone()
                .into_iter()
                .map(|msg_id| {
                    self.sent_msg_ids.remove(&msg_id);
                    msg_id
                })
                .collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        let result = match select.data() {
            Some(EDIT_RECORD) => CtxResult::Nothing,
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
