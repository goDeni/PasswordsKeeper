use std::sync::Arc;

use teloxide::{
    dispatching::dialogue::GetChatId,
    payloads::SendMessageSetters,
    requests::Requester,
    types::{CallbackQuery, Message},
    Bot,
};

use super::{create_repo::make_create_repo_keyboard, BotContext, HandlerResult, MyDialogue, State};

pub async fn main_state_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    context: Arc<BotContext>,
) -> HandlerResult {
    match context.store.exist(&msg.from().unwrap().id) {
        true => {
            unimplemented!()
        }
        false => {
            bot.send_message(msg.chat.id, "Репозиторий отсутствует")
                .reply_markup(make_create_repo_keyboard())
                .await?;

            dialogue.update(State::CreateRepoState).await?;
        }
    }

    Ok(())
}

pub async fn default_message_handler(bot: Bot, msg: Message) -> HandlerResult {
    log::debug!(
        "Defalt message handler called. chat_id: {}; from: {:?}",
        msg.chat.id,
        msg.from().map(|msg_from| msg_from.id)
    );

    bot.delete_message(msg.chat.id, msg.id).await?;
    Ok(())
}

pub async fn default_callback_handler(bot: Bot, query: CallbackQuery) -> HandlerResult {
    log::debug!(
        "Defalt callback handler called. chat_id: {:?}; from: {:?}",
        query.chat_id(),
        query.from.id
    );

    bot.answer_callback_query(query.id).await?;
    match query.message {
        Some(msg) => {
            bot.delete_message(msg.chat.id, msg.id).await?;
        }
        None => {}
    }

    Ok(())
}
