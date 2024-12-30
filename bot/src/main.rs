extern crate sec_store;

use bot::{
    bot::{handlers::build_handler, whitelist::Whitelist, BotContext, BotState},
    user_repo_factory::file::FileRepositoriesFactory,
};
use sec_store::repository::file::RecordsFileRepository;
use stated_dialogues::controller::ttl::track_dialog_ttl;
use std::env::current_dir;
use std::{fs::create_dir, path::Path, sync::Arc};
use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::{dptree, Bot, Dispatcher, LoggingErrorHandler},
};
use tempdir::TempDir;

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

    let curr_dir = current_dir().unwrap();

    let data_path = curr_dir.join("passwords_keeper_bot_data");
    let repositories_path = data_path.join("repositories");

    if !data_path.exists() {
        create_dir(data_path.clone()).unwrap();
    }
    if !repositories_path.exists() {
        create_dir(&repositories_path).unwrap();
    }

    let whitelist_file = curr_dir.join("whitelist");
    let whitelist = whitelist_file
        .exists()
        .then(|| {
            log::info!(
                "Found whitelist file \"{}\"",
                whitelist_file.to_str().unwrap()
            );
            Whitelist::read(&whitelist_file).unwrap()
        })
        .unwrap_or_else(Whitelist::new);

    log::info!("Whitelist members: {}", whitelist);

    log::info!("Starting bot...");
    let bot = Bot::from_env();

    let tmp_dir = TempDir::new_in(data_path.clone(), "tmp_").unwrap();
    let factory = FileRepositoriesFactory(repositories_path);
    let bot_context = Arc::new(BotContext::new(
        factory,
        bot.clone(),
        tmp_dir.path().to_path_buf(),
        whitelist,
    ));
    let documents_tempdir = Arc::new(TempDir::new_in(data_path, "documents_").unwrap());

    tokio::spawn(track_dialog_ttl(
        bot_context.dial.clone(),
        bot_context.bot_adapter.clone(),
        300,
        Some(format!("Диалог закрыт потому что не использовался {} секунд 🙈\nВведите /start чтобы инициировать новый диалог", 300)),
    ));
    Dispatcher::builder(
        bot,
        build_handler::<FileRepositoriesFactory, RecordsFileRepository>(),
    )
    .dependencies(dptree::deps![
        InMemStorage::<BotState>::new(),
        bot_context,
        documents_tempdir
    ])
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
    tmp_dir.close().unwrap();
}
