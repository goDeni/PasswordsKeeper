use std::io;

use clap::Parser;
use tui::cli::CliArgs;
use tui::{configure_data_dir, resolve_data_dir, App, AppConfig};

fn main() -> io::Result<()> {
    let args = CliArgs::parse();
    let data_dir = resolve_data_dir(args.data_dir);
    configure_data_dir(data_dir.clone());

    ratatui::run(|terminal| {
        let mut app = App::new(AppConfig { data_dir });
        app.run(terminal)
    })
}
