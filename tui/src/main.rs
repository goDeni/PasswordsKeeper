use std::io;

use tui::App;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| {
        let mut app = App::new();
        app.run(terminal)
    })
}
