from aiogram import Bot
from aiogram.types import CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup

from bot.dialogue.contexts.base import BaseContext, CallbackName
from bot.dialogue.contexts.inititlize_rep import InitializeRepCtx
from bot.dialogue.contexts.open_rep import OpenRepositoryCtx
from bot.user_reps import user_has_repository

_OPEN_REPOSITORY_CALLBACK = CallbackName("_OPEN_REPOSITORY_CALLBACK")


class HelloCtx(BaseContext):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__(bot, user_id)
        self._on_startup.append(self._send_hello_message())

    async def _send_hello_message(self):
        if not user_has_repository(self._user_id):
            self._set_new_ctx(InitializeRepCtx)
            return

        keyboard_markup = InlineKeyboardMarkup(row_width=3)
        keyboard_markup.row(
            InlineKeyboardButton(
                "Открыть репозиторий",
                callback_data=_OPEN_REPOSITORY_CALLBACK,
            )
        )
        await self._bot.send_message(
            self._user_id, "Репозиторий", reply_markup=keyboard_markup
        )
        self._set_callback(_OPEN_REPOSITORY_CALLBACK, self._open_repository_callback)

    async def _open_repository_callback(self, callback_query: CallbackQuery):
        self._set_new_ctx(OpenRepositoryCtx)
        await callback_query.message.delete()
