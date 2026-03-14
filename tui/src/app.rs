use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Clear, ListState, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

use crate::dialogues::Dialogue;
use crate::input::InputState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub data_dir: PathBuf,
}

pub struct App {
    pub config: AppConfig,
    pub screen: Box<dyn Dialogue>,
    pub input: Option<InputState>,
    pub list_state: ListState,
    pub error: Option<String>,
    pub success: Option<String>,
    pub exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new(AppConfig {
            data_dir: crate::repo::resolve_data_dir(None),
        })
    }
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        use crate::dialogues::WelcomeDialogue;
        Self {
            config,
            screen: Box::new(WelcomeDialogue::new(Some(0))),
            input: None,
            list_state: ListState::default(),
            error: None,
            success: None,
            exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            terminal.draw(|f| self.draw(f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn set_error(&mut self, e: impl ToString) {
        self.error = Some(e.to_string());
        self.success = None;
    }

    fn clear_error(&mut self) {
        self.error = None;
    }

    fn set_success(&mut self, msg: impl ToString) {
        self.success = Some(msg.to_string());
        self.error = None;
    }

    fn clear_success(&mut self) {
        self.success = None;
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if let Some(ref mut inp) = self.input {
            let block = Block::bordered()
                .title(" Input ")
                .border_set(border::ROUNDED)
                .border_style(Style::new().cyan());
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(inner);
            frame.render_widget(
                Paragraph::new(inp.prompt.as_str()).style(Style::new().dim()),
                chunks[0],
            );
            let mut p = Paragraph::new(inp.display())
                .style(Style::new().white())
                .block(Block::default());
            if inp.buffer.is_empty() && !inp.password_mode {
                p = p.style(Style::new().dark_gray());
            }
            frame.render_widget(p, chunks[1]);

            let instructions = if inp.password_mode {
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled("Enter", Style::new().cyan()),
                    Span::raw(" submit, "),
                    Span::styled("Ctrl+v", Style::new().cyan()),
                    Span::raw(" toggle visibility, "),
                    Span::styled("Esc", Style::new().cyan()),
                    Span::raw(" cancel"),
                ])
            } else {
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled("Enter", Style::new().cyan()),
                    Span::raw(" submit, "),
                    Span::styled("Esc", Style::new().cyan()),
                    Span::raw(" cancel"),
                ])
            };
            let bottom = Rect {
                y: area.y + area.height.saturating_sub(1),
                ..area
            };
            frame.render_widget(
                Paragraph::new(instructions).style(Style::new().dim()),
                bottom,
            );
            return;
        }

        self.screen.draw(frame, area);

        if let Some(ref err) = self.error {
            let overlay = centered_rect(60, 5, area);
            frame.render_widget(Clear, overlay);
            let block = Block::bordered()
                .title(" Error ")
                .border_set(border::ROUNDED)
                .border_style(Style::new().red());
            let inner = block.inner(overlay);
            frame.render_widget(block, overlay);
            let text = if err.len() > 56 {
                format!("{}...", &err[..53])
            } else {
                err.clone()
            };
            frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
        }

        if let Some(ref msg) = self.success {
            let overlay = centered_rect(60, 5, area);
            frame.render_widget(Clear, overlay);
            let block = Block::bordered()
                .title(" Success ")
                .border_set(border::ROUNDED)
                .border_style(Style::new().green());
            let inner = block.inner(overlay);
            frame.render_widget(block, overlay);
            let text = if msg.len() > 56 {
                format!("{}...", &msg[..53])
            } else {
                msg.clone()
            };
            frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
        }
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => self.handle_key(k),
            _ => {}
        }
        Ok(())
    }

    fn handle_key(&mut self, k: KeyEvent) {
        if self.error.is_some() {
            if k.code == KeyCode::Char(' ') || k.code == KeyCode::Enter || k.code == KeyCode::Esc {
                self.clear_error();
            }
            return;
        }

        if self.success.is_some() {
            if k.code == KeyCode::Char(' ') || k.code == KeyCode::Enter || k.code == KeyCode::Esc {
                self.clear_success();
            }
            return;
        }

        if let Some(ref mut inp) = self.input {
            match k.code {
                KeyCode::Enter => {
                    let value = inp.take();
                    self.input = None;
                    let result = self.screen.on_input_submit(value);
                    self.handle_dialogue_result(result);
                }
                KeyCode::Esc => {
                    self.input = None;
                    let result = self.screen.on_input_cancel();
                    self.handle_dialogue_result(result);
                }
                KeyCode::Char('v')
                    if inp.password_mode && k.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    inp.password_visible = !inp.password_visible;
                }
                KeyCode::Char(c) => inp.push_char(c),
                KeyCode::Backspace => inp.backspace(),
                _ => {}
            }
            return;
        }

        if k.code == KeyCode::Char('q') {
            self.exit = true;
        } else {
            let result = self.screen.handle_key(k);
            self.handle_dialogue_result(result);
        }
    }

    fn start_input(&mut self, prompt: &str, password: bool) {
        self.input = Some(InputState::new(prompt, password));
    }

    fn handle_dialogue_result(&mut self, result: crate::dialogues::DialogueResult) {
        use crate::dialogues::DialogueResult;
        match result {
            DialogueResult::NoOp => {}
            DialogueResult::ChangeScreen(dialogue) => {
                self.screen = dialogue;
            }
            DialogueResult::ChangeScreenAndStartInput {
                dialogue,
                prompt,
                password,
            } => {
                self.screen = dialogue;
                self.start_input(&prompt, password);
            }
            DialogueResult::StartInput { prompt, password } => {
                self.start_input(&prompt, password);
            }
            DialogueResult::Exit => {
                self.exit = true;
            }
            DialogueResult::Error(msg) => {
                self.set_error(msg);
            }
            DialogueResult::Success(msg) => {
                self.set_success(msg);
            }
        }
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(r.width * percent_x / 100)) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Min(0),
        ])
        .split(vertical[1]);
    horizontal[1]
}
