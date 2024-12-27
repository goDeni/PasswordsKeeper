use std::{collections::HashSet, marker::PhantomData};

use sec_store::repository::RecordsRepository;

use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;

use stated_dialogues::dialogues::{
    ButtonPayload, CtxResult, DialContext, Message, MessageId, Select, UserId,
};

use super::commands::{default_commands_handler, RESTORE_COMMAND};
use super::repository::{create_repo::CreateRepoDialogue, open_repo::OpenRepoDialogue};
use super::restore::RestoreDialogue;
use async_trait::async_trait;
use std::path::PathBuf;

pub struct HelloDialogue<T, R> {
    user_id: UserId,
    factory: T,
    sent_msg_ids: HashSet<MessageId>,
    tmp_directory: PathBuf,
    //
    phantom: PhantomData<R>,
}

impl<T, R> HelloDialogue<T, R> {
    pub fn new(user_id: UserId, factory: T, tmp_directory: PathBuf) -> Self {
        HelloDialogue {
            user_id,
            factory,
            sent_msg_ids: HashSet::new(),
            tmp_directory,
            //
            phantom: PhantomData,
        }
    }
}

const CREATE_REPO: &str = "1";
const OPEN_REPO: &str = "2";
const RESTORE_REPO: &str = "3";
const CANCEL_RESTORE: &str = "4";
const CONFIRM_RESTORE: &str = "5";

impl<F, R> HelloDialogue<F, R>
where
    R: RecordsRepository,
    F: RepositoriesFactory<R>,
{
    fn get_hello_buttons(&self) -> CtxResult {
        match self.factory.user_has_repository(&self.user_id.0) {
            false => CtxResult::Buttons(
                "Выберите действие".into(),
                vec![vec![(
                    ButtonPayload(CREATE_REPO.to_string()),
                    "Создать репозиторий".to_string(),
                )]],
            ),
            true => CtxResult::Buttons(
                "Репозиторий".into(),
                vec![vec![(
                    ButtonPayload(OPEN_REPO.to_string()),
                    "Открыть репозиторий".to_string(),
                )]],
            ),
        }
    }
}

#[async_trait]
impl<F, R> DialContext for HelloDialogue<F, R>
where
    R: RecordsRepository,
    F: RepositoriesFactory<R>,
{
    async fn init(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![self.get_hello_buttons()])
    }

    async fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    async fn handle_select(&mut self, select: Select) -> Result<Vec<CtxResult>> {
        let result: CtxResult = match (&select.user_id, select.data()) {
            (_, Some(OPEN_REPO)) => {
                CtxResult::NewCtx(Box::new(OpenRepoDialogue::new(self.factory.clone())))
            }
            (_, Some(CREATE_REPO)) => {
                CtxResult::NewCtx(Box::new(CreateRepoDialogue::new(self.factory.clone())))
            }
            (_, Some(CANCEL_RESTORE)) => self.get_hello_buttons(),
            (_, Some(RESTORE_REPO)) => CtxResult::Buttons(
                "Вы уверены?".into(),
                vec![
                    vec![(ButtonPayload(CONFIRM_RESTORE.into()), "✅ Да".to_string())],
                    vec![(ButtonPayload(CANCEL_RESTORE.into()), "❌ Нет".to_string())],
                ],
            ),
            (_, Some(CONFIRM_RESTORE)) => CtxResult::NewCtx(Box::new(RestoreDialogue::new(
                self.user_id.clone(),
                self.factory.clone(),
                self.tmp_directory.clone(),
            ))),
            _ => CtxResult::Nothing,
        };

        if let Some(msg_id) = &select.msg_id {
            self.sent_msg_ids.remove(msg_id);
        }

        Ok(vec![
            result,
            select
                .msg_id
                .map(|msg_id| CtxResult::RemoveMessages(vec![msg_id]))
                .unwrap_or(CtxResult::Nothing),
        ])
    }

    async fn handle_message(&mut self, input: Message) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(vec![input.id])])
    }

    async fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        Ok(match command.text() {
            Some(RESTORE_COMMAND) => {
                vec![
                    CtxResult::Buttons(
                        "Если у вас уже есть сохранённые пароли, они будут удалены".into(),
                        vec![
                            vec![(
                                ButtonPayload(RESTORE_REPO.to_string()),
                                "💾 Восстановить".into(),
                            )],
                            vec![(
                                ButtonPayload(CANCEL_RESTORE.to_string()),
                                "❌ Отменить".into(),
                            )],
                        ],
                    ),
                    CtxResult::RemoveMessages(vec![command.id]),
                    CtxResult::RemoveMessages(self.sent_msg_ids.drain().collect()),
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
