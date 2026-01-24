# PasswordsKeeper TUI

Terminal UI for PasswordsKeeper using [ratatui](https://ratatui.rs/) and [crossterm](https://docs.rs/crossterm).

## Run

```bash
cargo run -p tui
# or
just run-tui
```

## Data directory

- Default: `./passwords_keeper_tui_data` (relative to current working directory)
- Override: set `PASSWORDS_KEEPER_TUI_DATA` to a directory path

Repository file: `{data_dir}/repo`  
Backup output: `{data_dir}/backup.json` (when using Backup from the repository view)

## Controls

- **Welcome**: ↑/k ↓/j — move, Enter — select, q — quit
- **Repository list**: Same + `a` add record, `b` backup, `c` close, Enter on item — view record
- **Record view**: `e` edit, `d` delete, `b` back, q — quit. Delete asks for confirmation: **Y** to remove, **N** or **Esc** to cancel.
- **Edit record**: ↑/k ↓/j — select field, Enter — edit, Esc — cancel
- **Input prompts**: Type then Enter to submit, Esc to cancel. For password fields: v toggles visibility
