use teloxide::{requests::Requester, types::Message, Bot};

use super::{HandlerResult, MyDialogue, State};

pub async fn first_state(
    bot: Bot,
    me: teloxide::types::Me,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "first_state").await?;
    dialogue.update(State::SecondState).await?;
    Ok(())
}

pub async fn second_state(
    bot: Bot,
    me: teloxide::types::Me,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "second_state").await?;
    dialogue.update(State::ThirdState).await?;
    Ok(())
}

pub async fn third_state(
    bot: Bot,
    me: teloxide::types::Me,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "third_state").await?;
    dialogue.update(State::FirstState).await?;
    Ok(())
}
