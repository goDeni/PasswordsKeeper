from typing import Dict

from aiogram.types import CallbackQuery, Message

from bot.common import get_bot, get_dispatcher
from bot.dialogue.contexts.hello import HelloCtx
from bot.dialogue.user_dialog import UserDialog

dp = get_dispatcher()
bot = get_bot()

_DIALOGUES: Dict[int, UserDialog] = {}


@dp.callback_query_handler()
async def callbacks_handler(query: CallbackQuery):
    dialog = _get_user_dialog(query.from_user.id)
    await dialog.handle_callback(query.data, query)


@dp.message_handler()
async def messages_handler(message: Message):
    dialog = _get_user_dialog(message.from_user.id)
    await dialog.handle_message(message)


def _get_user_dialog(user_id: int) -> UserDialog:
    if user_id not in _DIALOGUES:
        _DIALOGUES[user_id] = UserDialog(bot, HelloCtx, user_id)

    return _DIALOGUES[user_id]
