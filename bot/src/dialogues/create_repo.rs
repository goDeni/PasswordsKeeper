use teloxide::{
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup},
    Bot,
};

use super::{HandlerResult, MyDialogue};

pub async fn create_repo_callback(
    bot: Bot,
    dialogue: MyDialogue,
    query: CallbackQuery,
) -> HandlerResult {
    bot.send_message(query.from.id, "Ok").await?;
    match query.message {
        Some(msg) => {
            bot.delete_message(query.from.id, msg.id).await?;
        }
        None => {}
    }

    dialogue.reset().await?;
    bot.answer_callback_query(query.id).await?;
    Ok(())
}

pub fn make_create_repo_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Открыть репозиторий",
        "_",
    )]])
}
