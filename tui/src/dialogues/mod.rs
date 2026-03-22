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

use sec_store::repository::RecordsRepository;

use crate::repo::RepositoryFactory;

pub trait Dialogue<F, R>: std::fmt::Debug
where
    F: RepositoryFactory<R>,
    R: RecordsRepository,
{
    fn draw(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect);
    fn handle_key(&mut self, k: crossterm::event::KeyEvent) -> DialogueResult<F, R>;
    fn on_input_submit(&mut self, value: String) -> DialogueResult<F, R>;
    fn on_input_cancel(&mut self) -> DialogueResult<F, R>;
    fn on_exit(&mut self) {}
}

#[derive(Debug)]
pub enum DialogueResult<F, R>
where
    F: RepositoryFactory<R>,
    R: RecordsRepository,
{
    NoOp,
    ChangeScreen(Box<dyn Dialogue<F, R>>),
    ChangeScreenAndStartInput {
        dialogue: Box<dyn Dialogue<F, R>>,
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
