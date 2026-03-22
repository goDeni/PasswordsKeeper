use std::io;

use clap::Parser;
use tui::cli::CliArgs;
use tui::{
    load_remote_repository_config, resolve_data_dir, resolve_repo_path, App, AppConfig,
    ConnectionMode, FileRepositoryFactory, RemoteRepositoryFactory,
};

fn main() -> io::Result<()> {
    let args = CliArgs::parse().validate().unwrap_or_else(|err| err.exit());
    match args.connection {
        ConnectionMode::File => {
            let repo_path = resolve_repo_path(args.repo_file);
            let data_dir = resolve_data_dir(&repo_path);
            let factory = FileRepositoryFactory::new(repo_path);

            ratatui::run(|terminal| {
                let mut app = App::new(AppConfig { data_dir }, factory);
                app.run(terminal)
            })
        }
        ConnectionMode::Remote => {
            let config = load_remote_repository_config(
                args.remote_config
                    .expect("remote config must exist after CLI validation"),
            )
            .map_err(io::Error::other)?;
            let factory = RemoteRepositoryFactory::new(config).map_err(io::Error::other)?;
            let data_dir = std::env::current_dir().unwrap_or_default();

            ratatui::run(|terminal| {
                let mut app = App::new(AppConfig { data_dir }, factory);
                app.run(terminal)
            })
        }
    }
}
