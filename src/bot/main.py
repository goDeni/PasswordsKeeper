import logging
from asyncio import gather
from typing import Dict

from aiogram import Bot, Dispatcher, executor
from aiogram.types import CallbackQuery, Message

from bot.config import get_bot_token
from bot.dialogue.contexts.hello import HelloCtx
from bot.dialogue.user_dialog import UserDialog

# Configure logging
logging.basicConfig(level=logging.INFO)

bot = Bot(token=get_bot_token())
dp = Dispatcher(bot)

_DIALOGUES: Dict[int, UserDialog] = {}


@dp.callback_query_handler()
async def _callbacks_handler(query: CallbackQuery):
    dialog = _get_user_dialog(query.from_user.id)
    await dialog.handle_callback(query.data, query)


@dp.message_handler(commands=["close"])
async def _close_command_handler(message: Message):
    _remove_user_dialog(message.from_user.id)
    await gather(
        message.answer("🌚"),
        message.delete(),
    )


@dp.message_handler(commands=["show"])
async def _show_command_handler(message: Message):
    dialog = _get_user_dialog(message.from_user.id)
    await dialog.handle_command(message)


@dp.message_handler()
async def _messages_handler(message: Message):
    dialog = _get_user_dialog(message.from_user.id)
    await dialog.handle_message(message)


def _get_user_dialog(user_id: int) -> UserDialog:
    if user_id not in _DIALOGUES:
        _DIALOGUES[user_id] = UserDialog(bot, HelloCtx, user_id)

    return _DIALOGUES[user_id]


def _remove_user_dialog(user_id: int):
    _DIALOGUES.pop(user_id, None)


def main():
    executor.start_polling(dp, skip_updates=True)


if __name__ == "__main__":
    main()
