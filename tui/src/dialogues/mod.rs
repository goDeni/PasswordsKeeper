pub mod add_record;
pub mod create_repo;
pub mod edit_record;
pub mod open_repo;
pub mod view_record;
pub mod view_repo;
pub mod welcome;

pub use add_record::AddRecordDialogue;
pub use view_record::ViewRecordDialogue;
pub use welcome::WelcomeDialogue;

pub trait Dialogue: std::fmt::Debug {
    fn draw(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect);
    fn handle_key(&mut self, k: crossterm::event::KeyEvent) -> DialogueResult;
    fn on_input_submit(&mut self, value: String) -> DialogueResult;
    fn on_input_cancel(&mut self) -> DialogueResult;
}

#[derive(Debug)]
pub enum DialogueResult {
    NoOp,
    ChangeScreen(Box<dyn Dialogue>),
    ChangeScreenAndStartInput {
        dialogue: Box<dyn Dialogue>,
        prompt: String,
        password: bool,
    },
    StartInput {
        prompt: String,
        password: bool,
    },
    Exit,
    Error(String),
    Success(String),
}
