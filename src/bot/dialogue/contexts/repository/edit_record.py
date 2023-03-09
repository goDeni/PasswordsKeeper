from enum import Enum, unique

from aiogram.types import (
    CallbackQuery,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    Message,
)
from aiogram.utils.exceptions import MessageNotModified

from bot.dialogue.contexts.base import BaseSubContext, CallbackName
from sec_store.record import Record

_EDIT_NAME = CallbackName("_EDIT_NAME")
_EDIT_DESCRIPTION = CallbackName("_EDIT_DESCRIPTION")
_EDIT_VALUE = CallbackName("_EDIT_VALUE")
_CANCEL_EDIT = CallbackName("_CANCEL_EDIT")
_SAVE = CallbackName("_SAVE")


@unique
class EditResult(Enum):
    CANCEL = 0
    SAVE = 1


@unique
class _Field(Enum):
    NAME = 0
    DESCRIPTION = 1
    VALUE = 2


class EditRecord(BaseSubContext[EditResult]):
    def __init__(self, *args, record: Record, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._record = record

        self._new_name = record.name
        self._new_description = record.description
        self._new_value = record.value

        self._changing_field: _Field | None = None
        self._enter_new_field_value_message: Message | None

        self._edit_rec_message: Message | None = None
        self._on_startup.append(self._send_or_update_edit_record_message())

    async def _send_or_update_edit_record_message(self):
        keyboard_markup = (
            InlineKeyboardMarkup()
            .row(InlineKeyboardButton("✏️ Название", callback_data=_EDIT_NAME))
            .row(InlineKeyboardButton("✏️ Описание", callback_data=_EDIT_DESCRIPTION))
            .row(InlineKeyboardButton("✏️ Значение", callback_data=_EDIT_VALUE))
            .row(InlineKeyboardButton("❌ Отменить", callback_data=_CANCEL_EDIT))
            .row(InlineKeyboardButton("💾 Сохранить", callback_data=_SAVE))
        )
        message_text = (
            f"Предпросмотр:\n"
            f"\n"
            f"Название: <code>{self._new_name}</code>\n"
            f"Описание: <code>{self._new_description}</code>\n"
            f"Значение: <code>{self._new_value}</code>\n"
        )

        if self._edit_rec_message is None:
            self._set_callback(_EDIT_NAME, self._edit_name_callback)
            self._set_callback(_EDIT_DESCRIPTION, self._edit_description_callback)
            self._set_callback(_EDIT_VALUE, self._edit_value_callback)
            self._set_callback(_CANCEL_EDIT, self._cancel_edit_callback)
            self._set_callback(_SAVE, self._save_callback)

        if self._edit_rec_message is None:
            self._edit_rec_message = await self._bot.send_message(
                self._user_id,
                text=message_text,
                parse_mode="HTML",
                reply_markup=keyboard_markup,
            )
            return

        try:
            self._edit_rec_message = await self._edit_rec_message.edit_text(
                text=message_text,
                parse_mode="HTML",
                reply_markup=keyboard_markup,
            )
        except MessageNotModified:
            pass

    async def _handle_message(self, message: Message):
        await message.delete()
        if self._enter_new_field_value_message:
            await self._enter_new_field_value_message.delete()
            self._enter_new_field_value_message = None

        match self._changing_field:
            case _Field.NAME:
                self._new_name = message.text
            case _Field.DESCRIPTION:
                self._new_description = message.text
            case _Field.VALUE:
                self._new_value = message.text
            case _:
                return

        self._changing_field = None
        await self._send_or_update_edit_record_message()

    async def _edit_name_callback(
        self, callback_query: CallbackQuery
    ):  # pylint: disable=unused-argument
        self._changing_field = _Field.NAME
        self._enter_new_field_value_message = await self._bot.send_message(
            self._user_id, "Введите новое название"
        )

    async def _edit_description_callback(
        self, callback_query: CallbackQuery
    ):  # pylint: disable=unused-argument
        self._changing_field = _Field.DESCRIPTION
        self._enter_new_field_value_message = await self._bot.send_message(
            self._user_id, "Введите новое описание"
        )

    async def _edit_value_callback(
        self, callback_query: CallbackQuery
    ):  # pylint: disable=unused-argument
        self._changing_field = _Field.VALUE
        self._enter_new_field_value_message = await self._bot.send_message(
            self._user_id, "Введите новое значение"
        )

    async def _cancel_edit_callback(self, callback_query: CallbackQuery):
        self._set_result(EditResult.CANCEL)
        await callback_query.message.delete()

    async def _save_callback(self, callback_query: CallbackQuery):
        self._record.name = self._new_name
        self._record.description = self._new_description
        self._record.value = self._new_value

        self._set_result(EditResult.SAVE)
        await callback_query.message.delete()
