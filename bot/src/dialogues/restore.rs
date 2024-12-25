use crate::dialogues::repository::view_repo::ViewRepoDialog;
use std::{collections::HashSet, marker::PhantomData};

use crate::dialogues::commands::default_commands_handler;
use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::{RepositoriesFactory, RepositoryLoadError};
use anyhow::Result;

use stated_dialogues::dialogues::{CtxResult, DialContext, Message, MessageId, Select, UserId};

use async_trait::async_trait;
use std::fs::{remove_file, rename};
use std::path::PathBuf;
use tempfile::Builder as TempFileBuilder;

pub struct RestoreDialogue<T, R> {
    user_id: UserId,
    factory: T,
    sent_msg_ids: HashSet<MessageId>,
    file: Option<PathBuf>,
    tmp_directory: PathBuf,
    //
    phantom: PhantomData<R>,
}

impl<T, R> RestoreDialogue<T, R> {
    pub fn new(user_id: UserId, factory: T, tmp_directory: PathBuf) -> Self {
        RestoreDialogue {
            user_id,
            factory,
            sent_msg_ids: HashSet::new(),
            //
            phantom: PhantomData,
            file: None,
            tmp_directory,
        }
    }
}

#[async_trait]
impl<F, R> DialContext for RestoreDialogue<F, R>
where
    R: RecordsRepository,
    F: RepositoriesFactory<R>,
{
    async fn init(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::Messages(vec![
            "Отправьте файл который хотите использовать для восстановления".into(),
        ])])
    }

    async fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        if let Some(file) = &self.file {
            remove_file(file)?;
            self.file = None;
        }

        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    async fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        if let Some(msg_id) = &select.msg_id {
            self.sent_msg_ids.remove(msg_id);
        }

        Ok(vec![select
            .msg_id
            .map(|msg_id| CtxResult::RemoveMessages(vec![msg_id]))
            .unwrap_or(CtxResult::Nothing)])
    }

    async fn handle_message(&mut self, input: Message) -> Result<Vec<CtxResult>> {
        let mut result: Vec<CtxResult> = vec![];

        match (input.document_file(), &self.file, input.text()) {
            (Some(document_file), _, _) => {
                self.sent_msg_ids.insert(input.id.clone());
                if let Some(file) = &self.file {
                    remove_file(file)?;
                    self.file = None;

                    result.push(CtxResult::Messages(vec!["Изменён загруженный файл".into()]));
                }
                result.push(CtxResult::Messages(vec!["Введите пароль от файла".into()]));

                let tmp_file = TempFileBuilder::new()
                    .prefix(&format!("document_{}_", self.user_id))
                    .tempfile_in(&self.tmp_directory)?;
                rename(document_file, &tmp_file)?;
                self.file = Some(tmp_file.path().to_path_buf());
                tmp_file.keep()?;
            }
            (None, Some(file), Some(msg)) => {
                result.push(CtxResult::RemoveMessages(vec![input.id.clone()]));
                match self.factory.load_user_repository(
                    &self.user_id.clone().into(),
                    msg.to_string(),
                    file,
                ) {
                    Ok(repo) => {
                        self.file = None;
                        result.push(CtxResult::NewCtx(Box::new(ViewRepoDialog::new(repo))));
                    }
                    Err(RepositoryLoadError::WrongPassword) => {
                        result.push(CtxResult::Messages(vec![
                            "Неверный пароль, попробуйте ещё раз".into(),
                        ]))
                    }
                    Err(err) => {
                        log::error!(
                            "Got error during repository load for user_id={}: {}",
                            self.user_id,
                            err
                        );
                        result.push(CtxResult::Messages(vec![
                            "Не удалось восстановить БД, убедитесь что отправлен верный файл"
                                .into(),
                        ]));
                    }
                }
            }
            _ => {
                result.push(CtxResult::RemoveMessages(vec![input.id]));
            }
        };

        Ok(result)
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
        true
    }
}
