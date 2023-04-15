import logging
from asyncio import gather
from functools import partial
from typing import Dict

from aiogram import Bot, Dispatcher, executor
from aiogram.types import CallbackQuery, Message

from bot.config import get_bot_token
from bot.dialogue.contexts.hello import HelloCtx
from dialog.dialog import Dialog
from dialog.types import CallbackName, Command

# Configure logging
logging.basicConfig(level=logging.INFO)

bot = Bot(token=get_bot_token())
dp = Dispatcher(bot)

_DIALOGUES: Dict[int, Dialog[Message]] = {}


@dp.callback_query_handler()
async def _callbacks_handler(query: CallbackQuery):
    dialog = _get_user_dialog(query.from_user.id)
    await dialog.handle_callback(CallbackName(query.data), query.message)


@dp.message_handler(commands=["close"])
async def _close_command_handler(message: Message):
    await gather(
        _remove_user_dialog(message.from_user.id),
        message.answer("🌚"),
        message.delete(),
    )


@dp.message_handler()
async def _messages_handler(message: Message):
    dialog = _get_user_dialog(message.from_user.id)
    if message.is_command():
        await dialog.handle_command(Command(message.get_command(True)), message)
    else:
        await dialog.handle_message(message)


def _get_user_dialog(user_id: int) -> Dialog[Message]:
    if user_id not in _DIALOGUES:
        _DIALOGUES[user_id] = Dialog(partial(HelloCtx, bot, user_id))

    return _DIALOGUES[user_id]


async def _remove_user_dialog(user_id: int):
    dialog = _DIALOGUES.pop(user_id, None)
    if dialog is not None:
        await dialog.shutdown()


def main():
    executor.start_polling(dp, skip_updates=True)
