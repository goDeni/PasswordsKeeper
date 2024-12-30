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
            CtxResult::Messages(vec![format!("Эта команда \"{}\" не поддерживается в этом диалоге", CANCEL_COMMAND).into()]),
            remove_msg,
        ],
        Some(BACKUP_COMMAND) => vec![
            CtxResult::Messages(vec!["Команда для создания бекапа поддерживается только в диалоге просмотра репозитория".into()]),
            remove_msg,
        ],
        Some(RESTORE_COMMAND) => vec![
            CtxResult::Messages(vec!["Команда восстановления из бекапа поддерживается только в диалоге открытия/создания репозитория".into()]),
            remove_msg,
        ],
        Some(cmd) => vec![CtxResult::Messages(vec![format!("Неизвестная команда \"{}\"", cmd).into()]), CtxResult::RemoveMessages(vec![command.id])],
        _ => vec![remove_msg]
    }
}
