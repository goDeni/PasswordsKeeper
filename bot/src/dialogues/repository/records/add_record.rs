use std::collections::HashSet;

use sec_store::record::Record;
use sec_store::repository::RecordsRepository;

use super::fields::{
    RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD,
};
use crate::dialogues::commands::CANCEL_COMMAND;
use crate::dialogues::repository::view_repo::ViewRepoDialog;
use anyhow::Result;
use stated_dialogues::dialogues::DialContext;
use stated_dialogues::dialogues::{CtxResult, Message, MessageId, Select};

#[derive(Clone)]
struct NewRecord {
    pub name: String,
    pub passwd: String,
    pub login: Option<String>,
    pub description: Option<String>,
}

impl NewRecord {
    pub fn new(name: String, passwd: String) -> Self {
        NewRecord {
            name,
            passwd,
            login: None,
            description: None,
        }
    }
}

impl From<NewRecord> for Record {
    fn from(val: NewRecord) -> Self {
        let mut fields: Vec<(String, String)> = vec![
            (RECORD_NAME_FIELD.into(), val.name),
            (RECORD_PASSWD_FIELD.into(), val.passwd),
        ];
        if let Some(login) = val.login {
            fields.push((RECORD_LOGIN_FIELD.into(), login))
        }
        if let Some(description) = val.description {
            fields.push((RECORD_DESCR_FIELD.into(), description))
        }

        Record::new(fields)
    }
}

#[derive(Clone)]
enum AddRecordState {
    Value,
    Name(String),
    Login(NewRecord),
    Description(NewRecord),
}

pub struct AddRecordDialog<T> {
    repo: T,
    state: AddRecordState,
    sent_msg_ids: HashSet<MessageId>,
}

impl<T> AddRecordDialog<T> {
    pub fn new(repo: T) -> Self {
        AddRecordDialog {
            repo,
            state: AddRecordState::Value,
            sent_msg_ids: HashSet::new(),
        }
    }
}

impl<T> DialContext for AddRecordDialog<T>
where
    T: RecordsRepository,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::Messages(vec!["Введите пароль".into()])])
    }

    fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        Ok(vec![select
            .msg_id
            .map(|msg_id| CtxResult::RemoveMessages(vec![msg_id]))
            .unwrap_or(CtxResult::Nothing)])
    }

    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match message.text {
            Some(text) => match self.state.clone() {
                AddRecordState::Value => {
                    self.state = AddRecordState::Name(text);
                    CtxResult::Messages(vec!["Введите название".into()])
                }
                AddRecordState::Name(passwd) => {
                    self.state = AddRecordState::Login(NewRecord::new(text, passwd));
                    CtxResult::Messages(vec!["Введите логин".into()])
                }
                AddRecordState::Login(mut new_record) => {
                    new_record.login = Some(text);
                    self.state = AddRecordState::Description(new_record);
                    CtxResult::Messages(vec!["Введите описание".into()])
                }
                AddRecordState::Description(mut new_record) => {
                    new_record.description = Some(text);
                    self.repo.add_record(new_record.into()).unwrap();
                    self.repo.save().map_err(|err| {
                        log::error!(
                            "Failed repository saving during new record saving for {:?}",
                            message.user_id
                        );
                        err
                    })?;

                    CtxResult::NewCtx(Box::new(ViewRepoDialog::new(self.repo.clone())))
                }
            },
            None => CtxResult::Nothing,
        };

        Ok(vec![CtxResult::RemoveMessages(vec![message.id]), result])
    }

    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        match command.text() {
            Some(CANCEL_COMMAND) => Ok(vec![
                CtxResult::RemoveMessages(vec![command.id]),
                CtxResult::NewCtx(Box::new(ViewRepoDialog::new(self.repo.clone()))),
            ]),
            _ => Ok(vec![CtxResult::RemoveMessages(vec![command.id])]),
        }
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<MessageId>) {
        msg_ids.into_iter().for_each(|msg_id| {
            self.sent_msg_ids.insert(msg_id);
        });
    }
}
