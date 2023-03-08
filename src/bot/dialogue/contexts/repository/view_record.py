from asyncio import create_task
from enum import Enum, unique
from typing import Tuple

from aiogram.types import (
    CallbackQuery,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    Message,
)

from bot.dialogue.contexts.base import BaseSubContext, CallbackName
from sec_store.record import Record, RecordId

_CLOSE_VIEW_CALLBACK = CallbackName("_CLOSE_VIEW_CALLBACK")
_EDIT_CALLBACK = CallbackName("_EDIT_CALLBACK")
_DELETE_CALLBACK = CallbackName("_DELETE_CALLBACK")


@unique
class RecordAction(Enum):
    EDIT = 1
    DELETE = 2


class ViewRecord(BaseSubContext[Tuple[RecordAction, RecordId] | None]):
    def __init__(self, *args, record: Record) -> None:
        super().__init__(*args)

        self._record = record
        create_task(
            self._send_view_record_message(),
            name=f"_send_view_record_message for {self._user_id=}",
        )

    async def _send_view_record_message(self):
        keyboard_markup = (
            InlineKeyboardMarkup()
            .row(InlineKeyboardButton("✏️", callback_data=_EDIT_CALLBACK))
            .row(InlineKeyboardButton("❌", callback_data=_DELETE_CALLBACK))
            .row(InlineKeyboardButton("Закрыть", callback_data=_CLOSE_VIEW_CALLBACK))
        )

        self._set_callback(_CLOSE_VIEW_CALLBACK, self._close_view_callback)
        self._set_callback(_DELETE_CALLBACK, self._delete_callback)
        self._set_callback(_EDIT_CALLBACK, self._edit_callback)

        await self._bot.send_message(
            self._user_id,
            (
                f"Название: <code>{self._record.name}</code>\n"
                f"Описание: <code>{self._record.description}</code>\n"
                f"Значение: <code>{self._record.value}</code>\n"
            ),
            parse_mode="HTML",
            reply_markup=keyboard_markup,
        )

    async def _close_view_callback(self, callback_query: CallbackQuery):
        self._set_result(None)
        await callback_query.message.delete()

    async def _delete_callback(self, callback_query: CallbackQuery):
        self._set_result((RecordAction.DELETE, self._record.id))
        await callback_query.message.delete()

    async def _edit_callback(self, callback_query: CallbackQuery):
        self._set_result((RecordAction.EDIT, self._record.id))
        await callback_query.message.delete()

    async def _handle_message(self, message: Message):
        await message.delete()
