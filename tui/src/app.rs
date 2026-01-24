use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
    DefaultTerminal, Frame,
};
use sec_store::repository::RecordsRepository;
use sec_store::{record::Record, repository::file::RecordsFileRepository};

use crate::fields::{
    RECORD_DESCR_FIELD, RECORD_LOGIN_FIELD, RECORD_NAME_FIELD, RECORD_PASSWD_FIELD,
};
use crate::input::InputState;
use crate::repo::{backup_path, create_repo, has_repo, open_repo, restore_repo};

type RecordId = String;

#[derive(Clone, Debug)]
pub enum Screen {
    Welcome,
    CreateRepo1,
    CreateRepo2(String),
    OpenRepo,
    RestorePath,
    RestorePassword(PathBuf),
    ViewRepo(RecordsFileRepository),
    ViewRecord(RecordsFileRepository, RecordId, bool), // bool = confirm_delete
    AddRecord1(RecordsFileRepository),
    AddRecord2(RecordsFileRepository, String),
    AddRecord3(RecordsFileRepository, String, String),
    AddRecord4(RecordsFileRepository, String, String, Option<String>),
    EditRecordSelect(RecordsFileRepository, RecordId),
    EditRecordValue(RecordsFileRepository, RecordId, String),
}

#[derive(Debug)]
pub struct App {
    pub screen: Screen,
    pub input: Option<InputState>,
    pub list_state: ListState,
    pub error: Option<String>,
    pub exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut list = ListState::default();
        list.select(Some(0));
        Self {
            screen: Screen::Welcome,
            input: None,
            list_state: list,
            error: None,
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
    }

    fn clear_error(&mut self) {
        self.error = None;
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
                    Span::styled("v", Style::new().cyan()),
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

        match &self.screen {
            Screen::Welcome => self.draw_welcome(frame, area),
            Screen::CreateRepo1 | Screen::CreateRepo2(_) => {
                self.draw_input_prompt(frame, area, "Create repository")
            }
            Screen::OpenRepo => self.draw_input_prompt(frame, area, "Open repository"),
            Screen::RestorePath => self.draw_input_prompt(frame, area, "Restore from backup"),
            Screen::RestorePassword(_) => {
                self.draw_input_prompt(frame, area, "Restore from backup")
            }
            Screen::ViewRepo(_) => self.draw_view_repo(frame, area),
            Screen::ViewRecord(_, _, _) => self.draw_view_record(frame, area),
            Screen::AddRecord1(_)
            | Screen::AddRecord2(_, _)
            | Screen::AddRecord3(_, _, _)
            | Screen::AddRecord4(_, _, _, _) => self.draw_input_prompt(frame, area, "Add record"),
            Screen::EditRecordSelect(_, _) | Screen::EditRecordValue(_, _, _) => {
                self.draw_edit_record(frame, area)
            }
        }

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
    }

    fn draw_input_prompt(&self, frame: &mut Frame, area: Rect, title: &str) {
        let block = Block::bordered()
            .title(format!(" {title} "))
            .border_set(border::ROUNDED);
        frame.render_widget(block, area);
    }

    fn draw_welcome(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" PasswordsKeeper ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().cyan());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut items = vec![
            ListItem::new("Create repository"),
            ListItem::new("Open repository"),
            ListItem::new("Restore from backup"),
            ListItem::new("Quit"),
        ];
        if !has_repo() {
            items[1] =
                ListItem::new("Open repository (none exists)").style(Style::new().dark_gray());
        }
        let list = List::new(items)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");
        let mut state = ListState::default();
        state.select(self.list_state.selected());
        frame.render_stateful_widget(list, inner, &mut state);

        let instructions = Line::from(vec![
            Span::raw(" "),
            Span::styled("↑/k", Style::new().cyan()),
            Span::raw(" up "),
            Span::styled("↓/j", Style::new().cyan()),
            Span::raw(" down "),
            Span::styled("Enter", Style::new().cyan()),
            Span::raw(" select "),
            Span::styled("q", Style::new().cyan()),
            Span::raw(" quit"),
        ]);
        let bottom = Rect {
            y: area.y + area.height.saturating_sub(1),
            ..area
        };
        frame.render_widget(
            Paragraph::new(instructions).style(Style::new().dim()),
            bottom,
        );
    }

    fn draw_view_repo(&mut self, frame: &mut Frame, area: Rect) {
        let Screen::ViewRepo(repo) = &self.screen else {
            return;
        };
        let block = Block::bordered()
            .title(" Repository ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().green());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let records = repo.get_records().unwrap_or_default();
        let mut rows: Vec<(RecordId, String)> = records
            .iter()
            .map(|r| {
                let name = r
                    .get_field_value(RECORD_NAME_FIELD)
                    .unwrap_or_else(|| "-".to_string());
                (r.id.clone(), name)
            })
            .collect();
        rows.sort_by(|a, b| a.1.cmp(&b.1));

        let mut items: Vec<ListItem> = rows
            .iter()
            .map(|(_, name)| ListItem::new(name.as_str()))
            .collect();
        items.push(ListItem::new("─── Add record"));
        items.push(ListItem::new("─── Backup"));
        items.push(ListItem::new("─── Close repository"));

        let list = List::new(items.clone())
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");
        let sel = self.list_state.selected().unwrap_or(0);
        let mut state = ListState::default();
        state.select(Some(sel.min(items.len().saturating_sub(1))));
        frame.render_stateful_widget(list, inner, &mut state);

        let instructions = Line::from(vec![
            Span::styled("Enter", Style::new().cyan()),
            Span::raw(" view "),
            Span::styled("a", Style::new().cyan()),
            Span::raw(" add "),
            Span::styled("b", Style::new().cyan()),
            Span::raw(" backup "),
            Span::styled("c", Style::new().cyan()),
            Span::raw(" close "),
            Span::styled("q", Style::new().cyan()),
            Span::raw(" quit"),
        ]);
        let bottom = Rect {
            y: area.y + area.height.saturating_sub(1),
            ..area
        };
        frame.render_widget(
            Paragraph::new(instructions).style(Style::new().dim()),
            bottom,
        );
    }

    fn draw_view_record(&self, frame: &mut Frame, area: Rect) {
        let Screen::ViewRecord(repo, rid, confirm_delete) = &self.screen else {
            return;
        };
        let mut repo = repo.clone();
        let block = Block::bordered()
            .title(" Record ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().yellow());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rec = match repo.get(rid) {
            Ok(Some(r)) => r.clone(),
            _ => {
                let p = Paragraph::new("Record not found.");
                frame.render_widget(p, inner);
                return;
            }
        };

        let mut lines = vec![
            format!(
                "Name: {}",
                rec.get_field_value(RECORD_NAME_FIELD).unwrap_or_default()
            ),
            format!(
                "Password: {}",
                rec.get_field_value(RECORD_PASSWD_FIELD).unwrap_or_default()
            ),
        ];
        if let Some(l) = rec.get_field_value(RECORD_LOGIN_FIELD) {
            lines.push(format!("Login: {l}"));
        }
        if let Some(d) = rec.get_field_value(RECORD_DESCR_FIELD) {
            lines.push(format!("Description: {d}"));
        }
        if *confirm_delete {
            lines.push(String::new());
            lines.push("Do you really want to remove this record? (Y/N)".to_string());
        }
        let text = lines.join("\n");
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);

        let instructions = if *confirm_delete {
            Line::from(vec![
                Span::styled("Y", Style::new().cyan()),
                Span::raw(" yes, remove "),
                Span::styled("N", Style::new().cyan()),
                Span::raw(" / "),
                Span::styled("Esc", Style::new().cyan()),
                Span::raw(" no, cancel "),
                Span::styled("q", Style::new().cyan()),
                Span::raw(" quit"),
            ])
        } else {
            Line::from(vec![
                Span::styled("e", Style::new().cyan()),
                Span::raw(" edit "),
                Span::styled("d", Style::new().cyan()),
                Span::raw(" delete "),
                Span::styled("b", Style::new().cyan()),
                Span::raw(" back "),
                Span::styled("q", Style::new().cyan()),
                Span::raw(" quit"),
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
    }

    fn draw_edit_record(&mut self, frame: &mut Frame, area: Rect) {
        let (repo, rid, field) = match &self.screen {
            Screen::EditRecordSelect(r, id) => (r.clone(), id.clone(), None),
            Screen::EditRecordValue(r, id, f) => (r.clone(), id.clone(), Some(f.clone())),
            _ => return,
        };
        let block = Block::bordered()
            .title(" Edit record ")
            .border_set(border::ROUNDED)
            .border_style(Style::new().yellow());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(f) = field {
            let mut r = repo.clone();
            if let Ok(Some(rec)) = r.get(&rid) {
                let val = rec.get_field_value(&f).unwrap_or_default();
                let label = match f.as_str() {
                    RECORD_NAME_FIELD => "Name",
                    RECORD_LOGIN_FIELD => "Login",
                    RECORD_DESCR_FIELD => "Description",
                    RECORD_PASSWD_FIELD => "Password",
                    _ => &f,
                };
                let text = format!("Editing {label}:\n{val}");
                frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
            }
        } else {
            let mut r = repo.clone();
            let rec = match r.get(&rid) {
                Ok(Some(x)) => x.clone(),
                _ => return,
            };
            let mut items = vec![ListItem::new("Name"), ListItem::new("Password")];
            if rec.get_field_value(RECORD_LOGIN_FIELD).is_some() {
                items.push(ListItem::new("Login"));
            }
            if rec.get_field_value(RECORD_DESCR_FIELD).is_some() {
                items.push(ListItem::new("Description"));
            }
            let list = List::new(items.clone())
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">> ");
            let sel = self.list_state.selected().unwrap_or(0);
            let mut state = ListState::default();
            state.select(Some(sel.min(items.len().saturating_sub(1))));
            frame.render_stateful_widget(list, inner, &mut state);
        }

        let instructions = Line::from(vec![
            Span::styled("Enter", Style::new().cyan()),
            Span::raw(" select "),
            Span::styled("Esc", Style::new().cyan()),
            Span::raw(" cancel "),
            Span::styled("q", Style::new().cyan()),
            Span::raw(" quit"),
        ]);
        let bottom = Rect {
            y: area.y + area.height.saturating_sub(1),
            ..area
        };
        frame.render_widget(
            Paragraph::new(instructions).style(Style::new().dim()),
            bottom,
        );
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

        if let Some(ref mut inp) = self.input {
            match k.code {
                KeyCode::Enter => {
                    let value = inp.take();
                    self.input = None;
                    self.on_input_submit(value);
                }
                KeyCode::Esc => {
                    self.input = None;
                    self.on_input_cancel();
                }
                KeyCode::Char('v') if inp.password_mode => {
                    inp.password_visible = !inp.password_visible;
                }
                KeyCode::Char(c) => inp.push_char(c),
                KeyCode::Backspace => inp.backspace(),
                _ => {}
            }
            return;
        }

        match k.code {
            KeyCode::Char('q') => self.exit = true,
            _ => self.handle_screen_key(k),
        }
    }

    fn on_input_submit(&mut self, value: String) {
        match std::mem::replace(&mut self.screen, Screen::Welcome) {
            Screen::CreateRepo1 => {
                if value.is_empty() {
                    self.set_error("Password cannot be empty");
                    self.screen = Screen::CreateRepo1;
                    self.start_input("Choose a password", true);
                    return;
                }
                self.screen = Screen::CreateRepo2(value);
                self.start_input("Repeat the password", true);
            }
            Screen::CreateRepo2(first) => {
                if value != first {
                    self.set_error("Passwords don't match");
                    self.screen = Screen::CreateRepo2(first);
                    self.start_input("Repeat the password", true);
                    return;
                }
                self.clear_error();
                match create_repo(first.clone()) {
                    Ok(repo) => {
                        self.screen = Screen::ViewRepo(repo);
                        self.init_list_state(0);
                    }
                    Err(e) => {
                        self.set_error(e);
                        self.screen = Screen::CreateRepo2(first);
                        self.start_input("Repeat the password", true);
                    }
                }
            }
            Screen::OpenRepo => {
                self.clear_error();
                match open_repo(value) {
                    Ok(repo) => {
                        self.screen = Screen::ViewRepo(repo);
                        self.init_list_state(0);
                    }
                    Err(e) => {
                        self.set_error(e);
                        self.screen = Screen::OpenRepo;
                        self.start_input("Enter password", true);
                    }
                }
            }
            Screen::RestorePath => {
                let path = PathBuf::from(value.trim());
                if path.exists() {
                    self.screen = Screen::RestorePassword(path);
                    self.start_input("Enter the file password", true);
                } else {
                    self.set_error("File does not exist");
                    self.screen = Screen::RestorePath;
                    self.start_input("Backup file path", false);
                }
            }
            Screen::RestorePassword(path) => {
                self.clear_error();
                match restore_repo(&path, value) {
                    Ok(repo) => {
                        self.screen = Screen::ViewRepo(repo);
                        self.init_list_state(0);
                    }
                    Err(e) => {
                        self.set_error(e);
                        self.screen = Screen::RestorePassword(path);
                        self.start_input("Enter the file password", true);
                    }
                }
            }
            Screen::AddRecord1(repo) => {
                if value.is_empty() {
                    self.set_error("Password cannot be empty");
                    self.screen = Screen::AddRecord1(repo);
                    self.start_input("Enter password", true);
                    return;
                }
                self.screen = Screen::AddRecord2(repo, value);
                self.start_input("Enter name", false);
            }
            Screen::AddRecord2(repo, pass) => {
                if value.is_empty() {
                    self.set_error("Name cannot be empty");
                    self.screen = Screen::AddRecord2(repo, pass);
                    self.start_input("Enter name", false);
                    return;
                }
                self.screen = Screen::AddRecord3(repo, pass, value);
                self.start_input("Enter login (or leave empty to skip)", false);
            }
            Screen::AddRecord3(repo, pass, name) => {
                let login = if value.is_empty() { None } else { Some(value) };
                self.screen = Screen::AddRecord4(repo, pass, name, login);
                self.start_input("Enter description (or leave empty to skip)", false);
            }
            Screen::AddRecord4(mut repo, pass, name, login) => {
                let desc = if value.is_empty() { None } else { Some(value) };
                let mut fields = vec![
                    (RECORD_NAME_FIELD.to_string(), name.clone()),
                    (RECORD_PASSWD_FIELD.to_string(), pass.clone()),
                ];
                if let Some(l) = login.clone() {
                    fields.push((RECORD_LOGIN_FIELD.to_string(), l));
                }
                if let Some(d) = desc {
                    fields.push((RECORD_DESCR_FIELD.to_string(), d));
                }
                let record = Record::new(fields);
                if let Err(e) = repo.add_record(record) {
                    self.set_error(e);
                } else if let Err(e) = repo.save() {
                    self.set_error(e);
                } else {
                    self.screen = Screen::ViewRepo(repo);
                    self.init_list_state(0);
                    return;
                }
                self.screen = Screen::AddRecord4(repo, pass, name, login);
                self.start_input("Enter description (or leave empty to skip)", false);
            }
            Screen::EditRecordValue(repo, rid, field) => {
                let mut r = repo.clone();
                if let Ok(Some(rec)) = r.get(&rid) {
                    let mut rec = rec.clone();
                    if rec.update_field(field.clone(), value).is_ok()
                        && r.update(rec).is_ok()
                        && r.save().is_ok()
                    {
                        self.screen = Screen::ViewRecord(r, rid, false);
                        return;
                    }
                }
                self.set_error("Update failed");
                self.screen = Screen::EditRecordValue(repo, rid, field);
                self.start_input("Enter new value", false);
            }
            s => {
                self.screen = s;
            }
        }
    }

    fn on_input_cancel(&mut self) {
        match &self.screen {
            Screen::CreateRepo1 | Screen::CreateRepo2(_) => self.screen = Screen::Welcome,
            Screen::OpenRepo | Screen::RestorePath | Screen::RestorePassword(_) => {
                self.screen = Screen::Welcome
            }
            Screen::AddRecord1(repo) => {
                let repo = repo.clone();
                self.screen = Screen::ViewRepo(repo);
                self.init_list_state(0);
            }
            Screen::AddRecord2(repo, _)
            | Screen::AddRecord3(repo, _, _)
            | Screen::AddRecord4(repo, _, _, _) => {
                let repo = repo.clone();
                self.screen = Screen::ViewRepo(repo);
                self.init_list_state(0);
            }
            Screen::EditRecordValue(repo, rid, _) => {
                let (repo, rid) = (repo.clone(), rid.clone());
                self.screen = Screen::EditRecordSelect(repo, rid);
            }
            _ => {}
        }
    }

    fn start_input(&mut self, prompt: &str, password: bool) {
        self.input = Some(InputState::new(prompt, password));
    }

    fn init_list_state(&mut self, sel: usize) {
        self.list_state.select(Some(sel));
    }

    fn handle_screen_key(&mut self, k: KeyEvent) {
        match &mut self.screen {
            Screen::Welcome => self.handle_welcome_key(k),
            Screen::ViewRepo(_) => self.handle_view_repo_key(k),
            Screen::ViewRecord(_, _, _) => self.handle_view_record_key(k),
            Screen::EditRecordSelect(_, _) | Screen::EditRecordValue(_, _, _) => {
                self.handle_edit_record_key(k)
            }
            _ => {}
        }
    }

    fn handle_welcome_key(&mut self, k: KeyEvent) {
        let n = 4;
        let sel = self.list_state.selected().unwrap_or(0);
        match k.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state
                    .select(Some(if sel == 0 { n - 1 } else { sel - 1 }));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select(Some((sel + 1) % n));
            }
            KeyCode::Enter => match sel {
                0 => {
                    self.screen = Screen::CreateRepo1;
                    self.start_input("Choose a password", true);
                }
                1 => {
                    if has_repo() {
                        self.screen = Screen::OpenRepo;
                        self.start_input("Enter password", true);
                    }
                }
                2 => {
                    self.screen = Screen::RestorePath;
                    self.start_input("Backup file path", false);
                }
                3 => self.exit = true,
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_view_repo_key(&mut self, k: KeyEvent) {
        let Screen::ViewRepo(repo) = &self.screen else {
            return;
        };
        let records = repo.get_records().unwrap_or_default();
        let mut rows: Vec<(RecordId, String)> = records
            .iter()
            .map(|r| {
                let name = r
                    .get_field_value(RECORD_NAME_FIELD)
                    .unwrap_or_else(|| "-".to_string());
                (r.id.clone(), name)
            })
            .collect();
        rows.sort_by(|a, b| a.1.cmp(&b.1));
        let n_rec = rows.len();
        let n = n_rec + 3; // Add, Backup, Close
        let sel = self
            .list_state
            .selected()
            .unwrap_or(0)
            .min(n.saturating_sub(1));

        match k.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state
                    .select(Some(if sel == 0 { n - 1 } else { sel - 1 }));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select(Some((sel + 1) % n));
            }
            KeyCode::Char('a') => {
                let repo = repo.clone();
                self.screen = Screen::AddRecord1(repo);
                self.start_input("Enter password", true);
            }
            KeyCode::Char('b') => {
                let repo = repo.clone();
                if let Ok(data) = repo.dump() {
                    if std::fs::write(backup_path(), data).is_ok() {
                        self.set_error(format!("Backup saved to {:?}", backup_path()));
                    } else {
                        self.set_error("Failed to write backup file");
                    }
                } else {
                    self.set_error("Backup failed");
                }
            }
            KeyCode::Char('c') => {
                self.screen = Screen::Welcome;
                self.init_list_state(0);
            }
            KeyCode::Enter => {
                if sel < n_rec {
                    let rid = rows[sel].0.clone();
                    let repo = repo.clone();
                    self.screen = Screen::ViewRecord(repo, rid, false);
                } else if sel == n_rec {
                    let repo = repo.clone();
                    self.screen = Screen::AddRecord1(repo);
                    self.start_input("Enter password", true);
                } else if sel == n_rec + 1 {
                    let repo = repo.clone();
                    if let Ok(data) = repo.dump() {
                        if std::fs::write(backup_path(), data).is_ok() {
                            self.set_error(format!("Backup saved to {:?}", backup_path()));
                        } else {
                            self.set_error("Failed to write backup file");
                        }
                    } else {
                        self.set_error("Backup failed");
                    }
                } else {
                    self.screen = Screen::Welcome;
                    self.init_list_state(0);
                }
            }
            _ => {}
        }
    }

    fn handle_view_record_key(&mut self, k: KeyEvent) {
        let Screen::ViewRecord(repo, rid, confirm_delete) = &self.screen else {
            return;
        };
        let (mut repo, rid) = (repo.clone(), rid.clone());
        match k.code {
            KeyCode::Char('y') | KeyCode::Char('Y') if *confirm_delete => {
                if repo.delete(&rid).is_ok() && repo.save().is_ok() {
                    self.screen = Screen::ViewRepo(repo);
                    self.init_list_state(0);
                } else {
                    self.set_error("Delete failed");
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc if *confirm_delete => {
                self.screen = Screen::ViewRecord(repo, rid, false);
            }
            KeyCode::Char('e') if !*confirm_delete => {
                self.screen = Screen::EditRecordSelect(repo, rid);
                self.init_list_state(0);
            }
            KeyCode::Char('d') if !*confirm_delete => {
                self.screen = Screen::ViewRecord(repo, rid, true);
            }
            KeyCode::Char('b') if !*confirm_delete => {
                self.screen = Screen::ViewRepo(repo);
                self.init_list_state(0);
            }
            _ => {}
        }
    }

    fn handle_edit_record_key(&mut self, k: KeyEvent) {
        match &mut self.screen {
            Screen::EditRecordSelect(repo, rid) => {
                let mut r = repo.clone();
                let rec = match r.get(rid) {
                    Ok(Some(x)) => x.clone(),
                    _ => return,
                };
                let mut fields = vec![RECORD_NAME_FIELD, RECORD_PASSWD_FIELD];
                if rec.get_field_value(RECORD_LOGIN_FIELD).is_some() {
                    fields.push(RECORD_LOGIN_FIELD);
                }
                if rec.get_field_value(RECORD_DESCR_FIELD).is_some() {
                    fields.push(RECORD_DESCR_FIELD);
                }
                let n = fields.len();
                let sel = self
                    .list_state
                    .selected()
                    .unwrap_or(0)
                    .min(n.saturating_sub(1));
                match k.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.list_state
                            .select(Some(if sel == 0 { n - 1 } else { sel - 1 }));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.list_state.select(Some((sel + 1) % n));
                    }
                    KeyCode::Enter => {
                        let f = fields[sel].to_string();
                        let (repo, rid) = (repo.clone(), rid.clone());
                        self.screen = Screen::EditRecordValue(repo.clone(), rid.clone(), f.clone());
                        let mut r = repo.clone();
                        let val = r
                            .get(&rid)
                            .ok()
                            .and_then(|o| o.map(|rec| rec.get_field_value(&f).unwrap_or_default()))
                            .unwrap_or_default();
                        self.start_input("Enter new value", f == RECORD_PASSWD_FIELD);
                        if let Some(inp) = &mut self.input {
                            inp.buffer = val;
                        }
                    }
                    KeyCode::Esc => {
                        let (repo, rid) = (repo.clone(), rid.clone());
                        self.screen = Screen::ViewRecord(repo, rid, false);
                    }
                    _ => {}
                }
            }
            Screen::EditRecordValue(_, _, _) => {}
            _ => {}
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
