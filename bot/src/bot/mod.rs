mod handlers;
mod interaction;
mod ttl;

use std::{collections::HashMap, marker::PhantomData};

use sec_store::repository::RecordsRepository;
use teloxide::{macros::BotCommands, types::UserId};

use crate::dialogues_controller::DialogueController;
use crate::user_repo_factory::RepositoriesFactory;

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
            factory,
            dial_ctxs: HashMap::new(),
            phantom: PhantomData,
        }
    }
}
