pub mod handlers;

use std::sync::Arc;
use std::{collections::HashMap, marker::PhantomData};

use async_trait::async_trait;
use sec_store::repository::RecordsRepository;
use teloxide::Bot;
use teloxide::{macros::BotCommands, types::UserId};
use tokio::sync::RwLock;

use crate::dialogues::hello::HelloDialogue;
use crate::user_repo_factory::RepositoriesFactory;
use anyhow::Result;
use stated_dialogues::controller::teloxide::TeloxideAdapter;
use stated_dialogues::controller::{CtxResult, DialCtxActions, DialogueController};

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
    pub dial: Arc<RwLock<DialContext<F, R>>>,
    pub bot_adapter: Arc<TeloxideAdapter>,
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
    pub fn new(factory: F, bot: Bot) -> Self {
        BotContext {
            dial: Arc::new(RwLock::new(DialContext {
                factory,
                dial_ctxs: HashMap::new(),
                phantom: PhantomData,
            })),
            bot_adapter: Arc::new(TeloxideAdapter::new(bot)),
        }
    }
}

#[async_trait]
impl<F: RepositoriesFactory<R>, R: RecordsRepository> DialCtxActions for DialContext<F, R> {
    async fn new_controller(&self, user_id: u64) -> Result<(DialogueController, Vec<CtxResult>)> {
        let context = HelloDialogue::<F, R>::new(user_id.into(), self.factory.clone());
        DialogueController::create(context).await
    }

    fn take_controller(&mut self, user_id: &u64) -> Option<DialogueController> {
        self.dial_ctxs.remove(&UserId(*user_id))
    }

    fn put_controller(&mut self, user_id: u64, controller: DialogueController) {
        self.dial_ctxs.insert(UserId(user_id), controller);
    }

    fn dialogues_list(&self) -> Vec<(&u64, &DialogueController)> {
        self.dial_ctxs
            .iter()
            .map(|(user_id, controller)| (&user_id.0, controller))
            .collect()
    }
}
