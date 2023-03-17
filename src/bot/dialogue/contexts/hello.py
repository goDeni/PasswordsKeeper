from aiogram import Bot
from aiogram.types import (
    CallbackQuery,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    Message,
)

from bot.dialogue.contexts.base import BaseContext, CallbackName
from bot.dialogue.contexts.commands import SHOW_COMMAND
from bot.dialogue.contexts.common import delete_messages
from bot.dialogue.contexts.inititlize_rep import InitializeRepCtx
from bot.dialogue.contexts.open_rep import OpenRepositoryCtx
from bot.user_reps import user_has_repository

_OPEN_REPOSITORY_CALLBACK = CallbackName("_OPEN_REPOSITORY_CALLBACK")


class HelloCtx(BaseContext):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__(bot, user_id)

        self._keyboard_message: Message | None = None

        self._on_startup.append(self._show_keyboard())
        self._commands_emitter.set_handler(SHOW_COMMAND, self._show_command)

    async def _show_command(self, message: Message):
        await delete_messages(message)
        await self._show_keyboard()

    async def _show_keyboard(self):
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

        await delete_messages(self._keyboard_message)
        self._keyboard_message = await self._bot.send_message(
            self._user_id, "Репозиторий", reply_markup=keyboard_markup
        )
        self._callbacks_emitter.set_handler(
            _OPEN_REPOSITORY_CALLBACK, self._open_repository_callback
        )

    async def _open_repository_callback(self, callback_query: CallbackQuery):
        self._set_new_ctx(OpenRepositoryCtx)
        await delete_messages(callback_query.message)
