use std::collections::HashSet;

use sec_store::{
    record::{Record, RecordId},
    repository::RecordsRepository,
};

use super::{
    fields::{
        record_as_message, RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD,
        RECORD_PASSWD_FIELD,
    },
    view_record::ViewRecordDialog,
};
use crate::{
    dialogues::{commands::CANCEL_COMMAND, repository::view_repo::ViewRepoDialog},
    stated_dialogues::{ButtonPayload, CtxResult, DialContext, Message, MessageId, Select},
};

use anyhow::{anyhow, Context, Result};

#[derive(Clone)]
enum DialogState {
    FieldEdit(String),
    WaitForSelect,
}

pub struct EditRecordDialog<T> {
    repo: T,
    record_id: RecordId,
    sent_msg_ids: HashSet<MessageId>,
    state: DialogState,
}

impl<T> EditRecordDialog<T> {
    pub fn new(record_id: RecordId, repo: T) -> Self {
        EditRecordDialog {
            repo,
            record_id,
            sent_msg_ids: HashSet::new(),
            state: DialogState::WaitForSelect,
        }
    }
}

const _CANCEL_EDIT: &str = "CANCEL_EDIT";
const _SAVE_RESULT: &str = "SAVE_RESULT";

impl<T> DialContext for EditRecordDialog<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        match self.repo.get(&self.record_id) {
            Some(record) => Ok(vec![get_edit_record_buttons(record)]),
            None => {
                log::warn!(
                    "Tried to view record that doesn't exist. record_id={}",
                    self.record_id
                );
                Ok(vec![CtxResult::NewCtx(Box::new(ViewRepoDialog::new(
                    self.repo.clone(),
                )))])
            }
        }
    }

    fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        match select
            .data
            .with_context(|| {
                format!(
                    "select.data is None !? user_id={}, msg_id={:?}",
                    select.user_id, select.msg_id
                )
            })?
            .as_str()
        {
            _CANCEL_EDIT => {
                self.repo.cancel()?;
                Ok(vec![CtxResult::NewCtx(Box::new(ViewRepoDialog::new(
                    self.repo.clone(),
                )))])
            }
            _SAVE_RESULT => {
                self.repo.save()?;
                Ok(vec![CtxResult::NewCtx(Box::new(ViewRecordDialog::new(
                    self.repo.clone(),
                    self.record_id.clone(),
                )))])
            }
            select_payload => {
                let field_name = match select_payload {
                    RECORD_DESCR_FIELD => Ok("описание"),
                    RECORD_LOGIN_FIELD => Ok("логин"),
                    RECORD_NAME_FIELD => Ok("название"),
                    RECORD_PASSWD_FIELD => Ok("пароль"),
                    unexpected_field => Err(anyhow!(
                        "Selected unexpected field '{}' by user {}",
                        unexpected_field,
                        select.user_id
                    )),
                }?;

                self.state = DialogState::FieldEdit(select_payload.to_string());
                Ok(vec![
                    CtxResult::RemoveMessages(
                        self.sent_msg_ids.drain().collect::<Vec<MessageId>>(),
                    ),
                    CtxResult::Messages(vec![format!(
                        "Введите новое значение для поля '{}'",
                        field_name
                    )
                    .into()]),
                ])
            }
        }
    }

    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        match (self.state.clone(), message.text) {
            (DialogState::FieldEdit(field), Some(msg_text)) => {
                let mut record = self
                    .repo
                    .get(&self.record_id)
                    .with_context(|| {
                        format!("Missed record {} in FieldEdit state", self.record_id)
                    })?
                    .clone();

                record.update_field(field, msg_text)?;
                let edit_buttons = get_edit_record_buttons(&record);

                self.repo.update(record)?;
                self.state = DialogState::WaitForSelect;
                Ok(vec![
                    CtxResult::RemoveMessages(
                        self.sent_msg_ids
                            .drain()
                            .chain(vec![message.id])
                            .collect::<Vec<MessageId>>(),
                    ),
                    edit_buttons,
                ])
            }
            _ => Ok(vec![CtxResult::RemoveMessages(vec![message.id])]),
        }
    }

    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        match (self.state.clone(), command.text()) {
            (_, Some(CANCEL_COMMAND)) => {
                self.repo.cancel()?;
                Ok(vec![
                    CtxResult::RemoveMessages(vec![command.id]),
                    CtxResult::NewCtx(Box::new(EditRecordDialog::new(
                        self.record_id.clone(),
                        self.repo.clone(),
                    ))),
                ])
            }
            (_, _) => Ok(vec![CtxResult::RemoveMessages(vec![command.id])]),
        }
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
}

fn get_edit_record_buttons(record: &Record) -> CtxResult {
    let mut button_rows: Vec<Vec<(ButtonPayload, String)>> =
        vec![vec![(RECORD_NAME_FIELD.into(), "✏️ Название".into())]];

    if record.get_field_value(RECORD_LOGIN_FIELD).is_some() {
        button_rows.push(vec![(RECORD_LOGIN_FIELD.into(), "✏️ Логин".into())])
    }

    if record.get_field_value(RECORD_DESCR_FIELD).is_some() {
        button_rows.push(vec![(RECORD_DESCR_FIELD.into(), "✏️ Описание".into())])
    }

    if record.get_field_value(RECORD_PASSWD_FIELD).is_some() {
        button_rows.push(vec![(RECORD_PASSWD_FIELD.into(), "✏️ Пароль".into())])
    }

    button_rows.extend(vec![
        vec![(_CANCEL_EDIT.into(), "❌ Отменить".into())],
        vec![(_SAVE_RESULT.into(), "💾 Сохранить".into())],
    ]);

    CtxResult::Buttons(record_as_message(record), button_rows)
}
