from enum import Enum, unique
from typing import Tuple

from aiogram.types import (
    CallbackQuery,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    Message,
)

from bot.dialogue.contexts.base import BaseContext, CallbackName
from bot.dialogue.contexts.commands import SHOW_COMMAND
from bot.dialogue.contexts.common import delete_messages
from sec_store.record import Record, RecordId

_CLOSE_VIEW_CALLBACK = CallbackName("_CLOSE_VIEW_CALLBACK")
_EDIT_CALLBACK = CallbackName("_EDIT_CALLBACK")
_DELETE_CALLBACK = CallbackName("_DELETE_CALLBACK")


@unique
class RecordAction(Enum):
    EDIT = 1
    DELETE = 2


class ViewRecord(BaseContext[Tuple[RecordAction, RecordId] | None]):
    def __init__(self, *args, record: Record) -> None:
        super().__init__(*args)

        self._record = record
        self._view_rec_message: Message | None = None

        self._on_startup.append(self._send_view_record_keyboard())
        self._on_shutdown.append(self._delete_messages())

        self._commands_emitter.set_handler(SHOW_COMMAND, self._handle_show_command)

    async def _delete_messages(self):
        await delete_messages(self._view_rec_message)

    async def _handle_show_command(self, message: Message):
        await delete_messages(message)
        await self._send_view_record_keyboard()

    async def _send_view_record_keyboard(self):
        keyboard_markup = (
            InlineKeyboardMarkup()
            .row(InlineKeyboardButton("✏️", callback_data=_EDIT_CALLBACK))
            .row(InlineKeyboardButton("❌", callback_data=_DELETE_CALLBACK))
            .row(InlineKeyboardButton("⬅️ Закрыть", callback_data=_CLOSE_VIEW_CALLBACK))
        )

        self._callbacks_emitter.set_handler(
            _CLOSE_VIEW_CALLBACK, self._close_view_callback
        )
        self._callbacks_emitter.set_handler(_DELETE_CALLBACK, self._delete_callback)
        self._callbacks_emitter.set_handler(_EDIT_CALLBACK, self._edit_callback)

        await delete_messages(self._view_rec_message)
        self._view_rec_message = await self._bot.send_message(
            self._user_id,
            (
                f"Название: <code>{self._record.name}</code>\n"
                f"Описание: <code>{self._record.description}</code>\n"
                f"Значение: <code>{self._record.value}</code>\n"
            ),
            parse_mode="HTML",
            reply_markup=keyboard_markup,
        )

    async def _close_view_callback(
        self, callback_query: CallbackQuery
    ):  # pylint: disable=unused-argument
        self._exit_from_ctx()

    async def _delete_callback(
        self, callback_query: CallbackQuery
    ):  # pylint: disable=unused-argument
        self._set_result((RecordAction.DELETE, self._record.id))

    async def _edit_callback(
        self, callback_query: CallbackQuery
    ):  # pylint: disable=unused-argument
        self._set_result((RecordAction.EDIT, self._record.id))
