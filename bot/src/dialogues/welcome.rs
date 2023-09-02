use teloxide::{
    payloads::SendMessageSetters,
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message},
    Bot,
};

use super::{HandlerResult, MyDialogue, State};

pub async fn first_state(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let keyboard = make_keyboard();
    bot.send_message(msg.chat.id, "first_state")
        .reply_markup(keyboard)
        .await?;
    dialogue.update(State::SecondState).await?;
    Ok(())
}

pub async fn second_state(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let keyboard = make_keyboard();
    bot.send_message(msg.chat.id, "second_state")
        .reply_markup(keyboard)
        .await?;
    dialogue.update(State::ThirdState).await?;
    Ok(())
}

pub async fn third_state(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "third_state").await?;
    dialogue.update(State::FirstState).await?;
    Ok(())
}

fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let buttons = ["First", "Second"];

    for versions in buttons.chunks(3) {
        let row = versions
            .iter()
            .map(|&version| InlineKeyboardButton::callback(version.to_owned(), version.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(version) = q.data {
        let state = dialogue.get().await?;
        log::info!("state: {:?}", state);

        let text = format!("You chose: {version}. State: {:?}", state);
        bot.answer_callback_query(q.id).await?;

        if let Some(Message { id, chat, .. }) = q.message {
            bot.edit_message_text(chat.id, id, text).await?;
        } else if let Some(id) = q.inline_message_id {
            bot.edit_message_text_inline(id, text).await?;
        }

        log::info!("You chose: {}", version);
    }

    Ok(())
}
