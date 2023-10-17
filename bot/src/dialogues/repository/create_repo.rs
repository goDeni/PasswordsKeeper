use std::{collections::HashSet, marker::PhantomData};

use sec_store::repository::RecordsRepository;

use crate::{dialogues::commands::CANCEL_COMMAND, user_repo_factory::RepositoriesFactory};
use anyhow::{Context, Result};

use super::open_repo::OpenRepoDialogue;
use crate::stated_dialogues::{CtxResult, DialContext, Message, MessageId, Select};

#[derive(Clone)]
enum CreationState {
    Disabled,
    WaitForPassword,
    WaitPasswordRepeat(String),
}

pub struct CreateRepoDialogue<F, R> {
    factory: F,
    creation_state: CreationState,
    sent_msg_ids: HashSet<MessageId>,
    //
    phantom: PhantomData<R>,
}

impl<F, R> CreateRepoDialogue<F, R> {
    pub fn new(factory: F) -> Self {
        CreateRepoDialogue {
            factory,
            creation_state: CreationState::Disabled,
            sent_msg_ids: HashSet::new(),
            phantom: PhantomData,
        }
    }
}

impl<F, R> DialContext for CreateRepoDialogue<F, R>
where
    R: RecordsRepository,
    F: RepositoriesFactory<R>,
{
    fn init(&mut self) -> Result<Vec<CtxResult>> {
        self.creation_state = CreationState::WaitForPassword;
        Ok(vec![CtxResult::Messages(vec!["Придумайте пароль".into()])])
    }

    fn shutdown(&mut self) -> Result<Vec<CtxResult>> {
        Ok(vec![CtxResult::RemoveMessages(
            self.sent_msg_ids.drain().collect(),
        )])
    }

    fn handle_select(&mut self, _select: Select) -> Result<Vec<CtxResult>> {
        Ok(vec![])
    }

    fn handle_message(&mut self, message: Message) -> Result<Vec<CtxResult>> {
        match (
            message
                .user_id
                .with_context(|| format!("Got message without user_id msg_id={}", message.id))?,
            message.text,
            self.creation_state.clone(),
        ) {
            (_, Some(input), CreationState::WaitForPassword) => {
                if input.is_empty() {
                    return Ok(vec![
                        CtxResult::RemoveMessages(vec![message.id]),
                        CtxResult::Messages(vec!["Вы ничего не ввели!".into()]),
                    ]);
                }
                self.creation_state = CreationState::WaitPasswordRepeat(input.to_string());
                Ok(vec![
                    CtxResult::RemoveMessages(vec![message.id]),
                    CtxResult::Messages(vec!["Повторите пароль".into()]),
                ])
            }
            (user_id, Some(input), CreationState::WaitPasswordRepeat(passwd)) => {
                if passwd.ne(&input) {
                    return Ok(vec![
                        CtxResult::RemoveMessages(vec![message.id]),
                        CtxResult::Messages(vec!["Неверный пароль. Попробуйте еще раз".into()]),
                    ]);
                }
                self.creation_state = CreationState::Disabled;
                match self
                    .factory
                    .initialize_user_repository(&user_id.clone().into(), passwd)
                {
                    Ok(mut repo) => {
                        repo.save().map_err(|err| {
                            log::error!("Failed repository saving for {user_id}: {err}");
                            err
                        })?;

                        Ok(vec![
                            CtxResult::RemoveMessages(vec![message.id]),
                            CtxResult::NewCtx(Box::new(OpenRepoDialogue::new(
                                self.factory.clone(),
                                user_id,
                            ))),
                        ])
                    }
                    Err(_) => {
                        log::warn!(
                            "An attempt to create the existing one repository for {user_id}"
                        );
                        Ok(vec![
                            CtxResult::RemoveMessages(vec![message.id]),
                            CtxResult::NewCtx(Box::new(OpenRepoDialogue::new(
                                self.factory.clone(),
                                user_id,
                            ))),
                        ])
                    }
                }
            }
            _ => Ok(vec![CtxResult::RemoveMessages(vec![message.id])]),
        }
    }

    fn handle_command(&mut self, command: Message) -> Result<Vec<CtxResult>> {
        match command.text() {
            Some(CANCEL_COMMAND) => Ok(vec![
                CtxResult::RemoveMessages(vec![command.id]),
                CtxResult::CloseCtx,
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
