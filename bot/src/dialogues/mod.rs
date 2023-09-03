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

use self::welcome::{callback_handler, first_state, second_state, third_state};

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    FirstState,
    SecondState,
    ThirdState,
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
        .branch(dptree::case![State::FirstState].endpoint(first_state))
        .branch(dptree::case![State::SecondState].endpoint(second_state))
        .branch(dptree::case![State::ThirdState].endpoint(third_state));

    let callbacks_hanlder = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
        .endpoint(callback_handler);

    dptree::entry()
        .branch(messages_hanler)
        .branch(callbacks_hanlder)
}
