from enum import Enum, unique

from aiogram.types import InlineKeyboardButton, InlineKeyboardMarkup, Message

from bot.dialogue.contexts.base import BaseContext
from bot.dialogue.contexts.commands import SHOW_COMMAND
from bot.dialogue.contexts.common import delete_messages
from dialog.context import CallbackName
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


_NEW_DESCRIPTION_TEXT = "Введите новое описание"
_NEW_VALUE_TEXT = "Введите новое значение"
_NEW_NAME_TEXT = "Введите новое название"


class EditRecord(BaseContext[EditResult]):
    def __init__(self, *args, record: Record, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._record = record

        self._new_name = record.name
        self._new_description = record.description
        self._new_value = record.value

        self._changing_field: _Field | None = None
        self._enter_new_field_value_message: Message | None = None

        self._edit_rec_message: Message | None = None

        self.add_on_startup(self._send_edit_record_messages)
        self.add_on_shutdown(self._delete_messages)

        self._commands_emitter.set_handler(SHOW_COMMAND, self._handle_show_command)

    async def _delete_messages(self):
        await delete_messages(
            self._enter_new_field_value_message, self._edit_rec_message
        )

    async def _handle_show_command(self, message: Message):
        await delete_messages(message)
        await self._send_edit_record_messages(self._changing_field)

    async def _send_edit_record_messages(self, changing_field: _Field | None = None):
        self._callbacks_emitter.remove_all_handlers()
        self._changing_field = changing_field

        keyboard_markup = InlineKeyboardMarkup()
        if changing_field is None:
            keyboard_markup.row(
                InlineKeyboardButton("✏️ Название", callback_data=_EDIT_NAME),
            ).row(
                InlineKeyboardButton("✏️ Описание", callback_data=_EDIT_DESCRIPTION),
            ).row(
                InlineKeyboardButton("✏️ Значение", callback_data=_EDIT_VALUE),
            )
            self._callbacks_emitter.set_handler(_EDIT_NAME, self._edit_name_callback)
            self._callbacks_emitter.set_handler(
                _EDIT_DESCRIPTION, self._edit_description_callback
            )
            self._callbacks_emitter.set_handler(_EDIT_VALUE, self._edit_value_callback)

        keyboard_markup.row(
            InlineKeyboardButton("❌ Отменить", callback_data=_CANCEL_EDIT),
        ).row(
            InlineKeyboardButton("💾 Сохранить", callback_data=_SAVE),
        )

        self._callbacks_emitter.set_handler(_CANCEL_EDIT, self._cancel_edit_callback)

        self._callbacks_emitter.set_handler(_SAVE, self._save_callback)
        message_text = (
            f"Предпросмотр:\n"
            f"\n"
            f"Название: <code>{self._new_name}</code>\n"
            f"Описание: <code>{self._new_description}</code>\n"
            f"Значение: <code>{self._new_value}</code>\n"
        )

        await delete_messages(
            self._edit_rec_message, self._enter_new_field_value_message
        )
        self._edit_rec_message = await self._bot.send_message(
            self._user_id,
            text=message_text,
            parse_mode="HTML",
            reply_markup=keyboard_markup,
        )

        changing_field_text = _NEW_NAME_TEXT
        match changing_field:
            case _Field.DESCRIPTION:
                changing_field_text = _NEW_DESCRIPTION_TEXT
            case _Field.VALUE:
                changing_field_text = _NEW_VALUE_TEXT
            case None:
                return

        self._enter_new_field_value_message = await self._bot.send_message(
            self._user_id, changing_field_text
        )

    async def _handle_message(self, message: Message):
        await delete_messages(message, self._enter_new_field_value_message)
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

        await self._send_edit_record_messages()

    async def _edit_name_callback(
        self, message: Message
    ):  # pylint: disable=unused-argument
        await self._send_edit_record_messages(_Field.NAME)

    async def _edit_description_callback(
        self, message: Message
    ):  # pylint: disable=unused-argument
        await self._send_edit_record_messages(_Field.DESCRIPTION)

    async def _edit_value_callback(
        self, message: Message
    ):  # pylint: disable=unused-argument
        await self._send_edit_record_messages(_Field.VALUE)

    async def _cancel_edit_callback(
        self, message: Message
    ):  # pylint: disable=unused-argument
        self._set_result(EditResult.CANCEL)

    async def _save_callback(self, message: Message):  # pylint: disable=unused-argument
        self._record.name = self._new_name
        self._record.description = self._new_description
        self._record.value = self._new_value

        self._set_result(EditResult.SAVE)
