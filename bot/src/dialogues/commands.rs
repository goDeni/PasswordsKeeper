use stated_dialogues::dialogues::{CtxResult, Message};

pub const RESET_COMMAND: &str = "/reset";
pub const CANCEL_COMMAND: &str = "/cancel";
pub const BACKUP_COMMAND: &str = "/backup";
pub const RESTORE_COMMAND: &str = "/restore";

pub fn default_commands_handler(command: Message) -> Vec<CtxResult> {
    match command.text() {
        Some(RESET_COMMAND) => vec![CtxResult::CloseCtx, CtxResult::RemoveMessages(vec![command.id])],
        Some(CANCEL_COMMAND) => vec![CtxResult::Messages(vec!["Эта команда не поддерживается в этом диалоге".into()])],
        Some(BACKUP_COMMAND) => vec![CtxResult::Messages(vec!["Команда для создания бекапа поддерживается только в диалоге просмотра репозитория".into()])],
        Some(RESTORE_COMMAND) => vec![CtxResult::Messages(vec!["Команда восстановления из бекапа поддерживается только в диалоге открытия/создания репозитория".into()])],
        Some(_) => vec![CtxResult::Messages(vec!["Неизвестная команда".into()])],
        _ => vec![CtxResult::RemoveMessages(vec![command.id])]
    }
}
