mod create_repo;
mod welcome;

use std::error::Error;

use sec_store::repository::file::RecordsFileRepository;
use teloxide::{
    dispatching::{dialogue::InMemStorage, DpHandlerDescription, HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{DependencyMap, Dialogue, Handler},
    types::{CallbackQuery, Message, Update},
};

use crate::{
    reps_store::store::RepositoriesStore, user_repo_factory::file::FileRepositoriesFactory,
};

use self::{
    create_repo::{create_repo_callback, handle_password_message_handler},
    welcome::{default_callback_handler, default_message_handler, main_state_handler},
};

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    MainState,
    CreateRepoState,
    CreateRepoStateEnterPass,
}

pub struct BotContext {
    pub store: RepositoriesStore<FileRepositoriesFactory, RecordsFileRepository>,
}

pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

pub fn build_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    let messages_hanler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::MainState].endpoint(main_state_handler))
        .branch(
            dptree::case![State::CreateRepoStateEnterPass]
                .endpoint(handle_password_message_handler),
        )
        .endpoint(default_message_handler);

    let callbacks_hanlder = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
        .branch(dptree::case![State::CreateRepoState].endpoint(create_repo_callback))
        .endpoint(default_callback_handler);

    dptree::entry()
        .branch(messages_hanler)
        .branch(callbacks_hanlder)
}
