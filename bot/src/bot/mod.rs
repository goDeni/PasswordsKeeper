pub mod handlers;
mod interaction;
pub mod ttl;

use std::{collections::HashMap, marker::PhantomData};

use sec_store::repository::RecordsRepository;
use teloxide::{macros::BotCommands, types::UserId};

use crate::dialogues::hello::HelloDialogue;
use crate::dialogues_controller::{CtxResult, DialogueController, NewDialController};
use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;

#[derive(Clone, Default, Debug)]
pub enum BotState {
    #[default]
    Default,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Remove and initialize dialog")]
    Reset,
    #[command(description = "Cancel dialog action")]
    Cancel,
}

pub struct BotContext<F: RepositoriesFactory<R>, R: RecordsRepository> {
    dial: DialContext<F, R>,
}

pub struct DialContext<F: RepositoriesFactory<R>, R: RecordsRepository> {
    pub factory: F,
    pub dial_ctxs: HashMap<UserId, DialogueController>,
    //
    phantom: PhantomData<R>,
}

impl<F, R> BotContext<F, R>
where
    F: RepositoriesFactory<R>,
    R: RecordsRepository,
{
    pub fn new(factory: F) -> Self {
        BotContext {
            dial: DialContext {
                factory,
                dial_ctxs: HashMap::new(),
                phantom: PhantomData,
            },
        }
    }
}

impl<F: RepositoriesFactory<R>, R: RecordsRepository> NewDialController for DialContext<F, R> {
    fn new_controller(&self, user_id: u64) -> Result<(DialogueController, Vec<CtxResult>)> {
        let context = HelloDialogue::<F, R>::new(user_id.into(), self.factory.clone());
        DialogueController::create(context)
    }

    fn take_controller(&mut self, user_id: &u64) -> Option<DialogueController> {
        self.dial_ctxs.remove(&UserId(*user_id))
    }

    fn put_controller(&mut self, user_id: u64, controller: DialogueController) {
        self.dial_ctxs.insert(UserId(user_id), controller);
    }
}
