mod welcome;

use std::error::Error;

use teloxide::{
    dispatching::{dialogue::InMemStorage, DpHandlerDescription, HandlerExt, UpdateFilterExt},
    dptree,
    prelude::{DependencyMap, Dialogue, Handler},
    types::{Message, Update},
};

use self::welcome::{first_state, second_state, third_state};

#[derive(Clone, Default)]
pub enum State {
    #[default]
    FirstState,
    SecondState,
    ThirdState,
}

pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

pub fn build_handler(
) -> Handler<'static, DependencyMap, Result<(), Box<dyn Error + Send + Sync>>, DpHandlerDescription>
{
    Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::FirstState].endpoint(first_state))
        .branch(dptree::case![State::SecondState].endpoint(second_state))
        .branch(dptree::case![State::ThirdState].endpoint(third_state))
}
