use std::io;

use clap::Parser;
use tui::cli::CliArgs;
use tui::{configure_repo_path, resolve_data_dir, resolve_repo_path, App, AppConfig};

fn main() -> io::Result<()> {
    let args = CliArgs::parse();
    let repo_path = resolve_repo_path(args.repo_file);
    let data_dir = resolve_data_dir(&repo_path);
    configure_repo_path(repo_path);

    ratatui::run(|terminal| {
        let mut app = App::new(AppConfig { data_dir });
        app.run(terminal)
    })
}
