use stated_dialogues::dialogues::{CtxResult, Message};

pub const RESET_COMMAND: &str = "/reset";
pub const CANCEL_COMMAND: &str = "/cancel";
pub const BACKUP_COMMAND: &str = "/backup";
pub const RESTORE_COMMAND: &str = "/restore";

pub fn default_commands_handler(command: Message) -> Vec<CtxResult> {
    let remove_msg = CtxResult::RemoveMessages(vec![command.id.clone()]);

    match command.text() {
        Some(RESET_COMMAND) => vec![CtxResult::CloseCtx, remove_msg],
        Some(CANCEL_COMMAND) => vec![
            CtxResult::Messages(vec![format!("The \"{}\" command is not supported in this dialog", CANCEL_COMMAND).into()]),
            remove_msg,
        ],
        Some(BACKUP_COMMAND) => vec![
            CtxResult::Messages(vec!["The backup command is only supported in the repository view dialog".into()]),
            remove_msg,
        ],
        Some(RESTORE_COMMAND) => vec![
            CtxResult::Messages(vec!["The restore from backup command is only supported in the open/create repository dialog".into()]),
            remove_msg,
        ],
        Some(cmd) => vec![CtxResult::Messages(vec![format!("Unknown command \"{}\"", cmd).into()]), CtxResult::RemoveMessages(vec![command.id])],
        _ => vec![remove_msg]
    }
}
