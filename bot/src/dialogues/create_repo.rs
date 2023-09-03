use std::sync::Arc;

use async_mutex::Mutex;
use sec_store::repository::RecordsRepository;
use teloxide::{
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message},
    Bot,
};

use crate::user_repo_factory::RepositoryAlreadyExist;

use super::{BotContext, HandlerResult, MyDialogue, State};

pub async fn create_repo_callback(
    bot: Bot,
    dialogue: MyDialogue,
    query: CallbackQuery,
) -> HandlerResult {
    bot.send_message(query.from.id, "Придумайте пароль").await?;
    match query.message {
        Some(msg) => {
            bot.delete_message(query.from.id, msg.id).await?;
        }
        None => {}
    }

    dialogue.update(State::CreateRepoStateEnterPass).await?;
    bot.answer_callback_query(query.id).await?;
    Ok(())
}

pub async fn handle_password_message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    context: Arc<Mutex<BotContext>>,
) -> HandlerResult {
    let user_id = &msg.from().unwrap().id;

    bot.delete_message(msg.chat.id, msg.id).await?;
    match msg.text().map(|text| text.trim()) {
        Some(text) => {
            match text.len() {
                0 => {
                    bot.send_message(msg.chat.id, "Вы ничего не ввели. Введите пароль")
                        .await?;
                }
                _ => {
                    let mut ctx = context.lock().await;
                    match ctx.store.init(user_id, text.to_string()) {
                        Ok(repo) => {
                            match repo.lock().await.save() {
                                Ok(_) => {
                                    bot.send_message(msg.chat.id, "Repo created!").await?;
                                    dialogue.reset().await?;
                                    // TODO SWITCH STATE
                                }
                                Err(err) => {
                                    log::error!("Failed to attempt create repo {}", err);
                                    bot.send_message(
                                        msg.chat.id,
                                        "Не удалось создать репозиторий :(",
                                    )
                                    .await?;
                                }
                            }
                        }
                        Err(RepositoryAlreadyExist) => {
                            bot.send_message(msg.chat.id, "Репозиторий уже существует")
                                .await?;
                            dialogue.reset().await?;
                        }
                    }
                }
            };
        }
        None => {
            bot.send_message(msg.chat.id, "Вы ничего не ввели. Введите пароль")
                .await?;
        }
    };

    Ok(())
}

pub fn make_create_repo_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Открыть репозиторий",
        "_",
    )]])
}
