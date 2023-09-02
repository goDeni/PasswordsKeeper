extern crate sec_store;

use std::{path::Path, fs::create_dir, sync::Arc};

use async_mutex::Mutex;
use bot::{dialogues::{build_handler, State}, reps_store::store::RespsitoriesStore, user_repo_factory::file::FileRepositoriesFactory};
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
        .parse_filters(&std::env::var(&"RUST_LOG").unwrap_or("DEBUG".to_string()))
        .init();

    let data_path = Path::new("./.passwords_keeper_data");
    if !data_path.exists() {
        create_dir(data_path).unwrap();
    }

    log::info!("Starting bot...");
    let bot = Bot::from_env();

    let store = RespsitoriesStore::new(
        Box::new(FileRepositoriesFactory(data_path.to_path_buf()))
    );

    Dispatcher::builder(bot, build_handler())
        .dependencies(dptree::deps![
            InMemStorage::<State>::new()
            // Arc::new(Mutex::new(Box::new(store)))
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
}
