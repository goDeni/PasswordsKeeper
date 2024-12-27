extern crate sec_store;

use bot::{
    bot::{handlers::build_handler, BotContext, BotState},
    user_repo_factory::file::FileRepositoriesFactory,
};
use sec_store::repository::file::RecordsFileRepository;
use stated_dialogues::controller::ttl::track_dialog_ttl;
use std::collections::HashSet;
use std::env::current_dir;
use std::fs::File;
use std::io::prelude::Read;
use std::io::Result;
use std::{fs::create_dir, path::Path, sync::Arc};
use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::{dptree, Bot, Dispatcher, LoggingErrorHandler},
    types::UserId,
};
use tempdir::TempDir;

fn read_whitelist<P: AsRef<Path>>(file: P) -> Result<HashSet<UserId>> {
    let mut data = String::new();
    File::open(file)?.read_to_string(&mut data)?;

    Ok(HashSet::from_iter(
        data.lines()
            .map(|line| line.trim())
            .filter(|line| line.len().gt(&0))
            .filter_map(|line| {
                line.parse::<u64>().map_or_else(
                    |err| {
                        log::warn!("Failed line parse \"{}\": {}", line, err);
                        None
                    },
                    |num| Some(UserId(num)),
                )
            }),
    ))
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
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("DEBUG".to_string()))
        .init();

    let data_path = Path::new("./.passwords_keeper_bot_data");
    let repositories_path = data_path.join("repositories");

    if !data_path.exists() {
        create_dir(data_path).unwrap();
    }
    if !repositories_path.exists() {
        create_dir(&repositories_path).unwrap();
    }

    let whitelist_file = current_dir().unwrap().join("whitelist");
    let whitelist = whitelist_file
        .exists()
        .then(|| {
            log::info!(
                "Found whitelist file \"{}\"",
                whitelist_file.to_str().unwrap()
            );
            read_whitelist(&whitelist_file).unwrap()
        })
        .unwrap_or_else(HashSet::new);

    log::info!(
        "Whitelist members: {}",
        whitelist
            .iter()
            .map(|v| v.to_string())
            .reduce(|a, b| format!("'{}', '{}'", a, b))
            .map_or("[]".to_string(), |v| format!("[{}]", v))
    );

    log::info!("Starting bot...");
    let bot = Bot::from_env();

    let tmp_dir = TempDir::new_in(data_path, "tmp_").unwrap();
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
