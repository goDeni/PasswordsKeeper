extern crate sec_store;

use std::path::Path;

use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, types::Update};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;
#[derive(Clone, Default)]
pub enum State {
    #[default]
    FirstState,
    SecondState,
    ThirdState,
}

#[tokio::main]
async fn main() {
    {
        let env_file = Path::new(".env");
        if env_file.exists() {
            dotenv::from_filename(".env").unwrap();
        }
    }
    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&std::env::var(&"RUST_LOG").unwrap_or("DEBUG".to_string()))
        .init();

    log::info!("Starting bot...");
    let bot = Bot::from_env();

    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::FirstState].endpoint(first_state))
        .branch(dptree::case![State::SecondState].endpoint(second_state))
        .branch(dptree::case![State::ThirdState].endpoint(third_state));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .default_handler(|upd| async move {
            log::warn!("Unhandled update: {:?}", upd);
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn first_state(
    bot: Bot,
    me: teloxide::types::Me,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "first_state").await?;
    dialogue.update(State::SecondState).await?;
    Ok(())
}

async fn second_state(
    bot: Bot,
    me: teloxide::types::Me,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "second_state").await?;
    dialogue.update(State::ThirdState).await?;
    Ok(())
}

async fn third_state(
    bot: Bot,
    me: teloxide::types::Me,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "third_state").await?;
    dialogue.update(State::FirstState).await?;
    Ok(())
}
