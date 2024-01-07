extern crate sec_store;

use bot::{
    bot::{handlers::build_handler, BotContext, BotState},
    dialogues_controller::ttl::track_dialog_ttl,
    user_repo_factory::file::FileRepositoriesFactory,
};
use sec_store::repository::file::RecordsFileRepository;
use std::{fs::create_dir, path::Path, sync::Arc};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

#[tokio::main]
async fn main() {
    {
        let env_file = Path::new(".env");
        if env_file.exists() {
            dotenv::from_filename(".env").unwrap();
        }
    }
    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("DEBUG".to_string()))
        .init();

    let data_path = Path::new("./.passwords_keeper_bot_data");
    if !data_path.exists() {
        create_dir(data_path).unwrap();
    }

    log::info!("Starting bot...");
    let bot = Bot::from_env();

    let factory = FileRepositoriesFactory(data_path.to_path_buf());
    let bot_context = Arc::new(BotContext::new(factory, bot.clone()));

    tokio::spawn(track_dialog_ttl(
        bot_context.dial.clone(),
        bot_context.bot_adapter.clone(),
        300,
    ));
    Dispatcher::builder(
        bot,
        build_handler::<FileRepositoriesFactory, RecordsFileRepository>(),
    )
    .dependencies(dptree::deps![InMemStorage::<BotState>::new(), bot_context])
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
