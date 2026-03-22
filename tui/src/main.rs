use std::io;

use clap::Parser;
use tui::cli::CliArgs;
use tui::{
    configure_repository_source, load_remote_repository_config, resolve_data_dir,
    resolve_repo_path, App, AppConfig, ConnectionMode, RepositorySource,
};

fn main() -> io::Result<()> {
    let args = CliArgs::parse().validate().unwrap_or_else(|err| err.exit());
    let data_dir = match args.connection {
        ConnectionMode::File => {
            let repo_path = resolve_repo_path(args.repo_file);
            let data_dir = resolve_data_dir(&repo_path);
            configure_repository_source(RepositorySource::File { repo_path });
            data_dir
        }
        ConnectionMode::Remote => {
            let config = load_remote_repository_config(
                args.remote_config
                    .expect("remote config must exist after CLI validation"),
            )
            .map_err(io::Error::other)?;
            configure_repository_source(RepositorySource::Remote(config));
            std::env::current_dir().unwrap_or_default()
        }
    };

    ratatui::run(|terminal| {
        let mut app = App::new(AppConfig { data_dir });
        app.run(terminal)
    })
}
