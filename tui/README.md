# PasswordsKeeper TUI

Terminal UI for PasswordsKeeper using [ratatui](https://ratatui.rs/) and [crossterm](https://docs.rs/crossterm).

## Run

```bash
cargo run -p tui
# or
just run-tui
# or
cargo run -p tui -- --data-dir /path/to/passwords_keeper_tui_data
```

## Data directory

- Default: `./passwords_keeper_tui_data` (relative to current working directory)
- Override: set `PASSWORDS_KEEPER_TUI_DATA` to a directory path
- CLI override: pass `--data-dir /path/to/dir`
- Precedence: `--data-dir` overrides `PASSWORDS_KEEPER_TUI_DATA`

Repository file: `{data_dir}/repo`

## CLI parameters

- `--data-dir <PATH>`: Use `PATH` as the TUI data directory. This overrides `PASSWORDS_KEEPER_TUI_DATA`.

## Requirements

- **Clipboard support**: On Linux/Wayland, requires `wl-clipboard` package installed for password copying functionality (`wl-copy` command)

## Controls

- **Welcome**: ↑/k ↓/j — move, Enter — select, q — quit
- **Repository list**: 
  - `/` start search/filter (filter as you type)
  - ↑/k ↓/j — navigate, `a` add record, `c` close, Enter on item — view record
  - Search filters by name and login fields (case-insensitive)
  - `Esc` while searching cancels search and shows all records
- **Record view**: 
  - `e` edit, `c` copy password to clipboard, `d` delete, `b` back, q — quit
  - `Ctrl+v` toggle password visibility (password is hidden by default)
  - Delete asks for confirmation: **Y** to remove, **N** or **Esc** to cancel
- **Edit record**: ↑/k ↓/j — select field, Enter — edit, Esc — cancel
- **Input prompts**: Type then Enter to submit, Esc to cancel. For password fields: Ctrl+v toggles visibility
- **Messages**: Success and error messages can be dismissed with Space, Enter, or Esc

## Dialogues Structure

The TUI uses a dialogue-based architecture where each screen is implemented as a separate dialogue struct that implements the `Dialogue` trait. This design allows the `App` structure to remain abstract from specific screen implementations, working only with the `Dialogue` trait interface.

### Dialogue Trait

All dialogues implement the `Dialogue` trait with four methods:

- `draw(&mut self, frame: &mut Frame, area: Rect)` — Renders the dialogue UI
- `handle_key(&mut self, k: KeyEvent) -> DialogueResult` — Handles keyboard input when not in input mode
- `on_input_submit(&mut self, value: String) -> DialogueResult` — Called when user submits input (Enter)
- `on_input_cancel(&mut self) -> DialogueResult` — Called when user cancels input (Esc)

### DialogueResult Enum

Dialogues communicate actions back to `App` through `DialogueResult`:

- `NoOp` — No action needed
- `ChangeScreen(Box<dyn Dialogue>)` — Switch to a different dialogue
- `ChangeScreenAndStartInput { dialogue, prompt, password }` — Switch dialogue and immediately start input
- `StartInput { prompt, password }` — Start an input prompt
- `Exit` — Exit the application
- `Error(String)` — Report an error message

### Available Dialogues

Each dialogue is in its own file under `src/dialogues/`:

- **`WelcomeDialogue`** (`welcome.rs`) — Main menu with options to create/open repository or quit
- **`CreateRepoDialogue`** (`create_repo.rs`) — Two-step password creation for new repository
- **`OpenRepoDialogue`** (`open_repo.rs`) — Password prompt to unlock existing repository
- **`ViewRepoDialogue`** (`view_repo.rs`) — List of records in the repository
- **`ViewRecordDialogue`** (`view_record.rs`) — View details of a single record
- **`AddRecordDialogue`** (`add_record.rs`) — Multi-step form to add a new record
- **`EditRecordDialogue`** (`edit_record.rs`) — Edit existing record fields

### Architecture Benefits

- **Separation of Concerns**: Each dialogue manages its own state and UI rendering
- **Input Handling**: Dialogues handle their own input flow, determining when to request input and how to process it
- **Type Abstraction**: `App` uses `Box<dyn Dialogue>`, allowing easy addition of new dialogues without modifying core app logic
- **Modularity**: Each dialogue is self-contained in its own file, making the codebase easier to navigate and maintain
