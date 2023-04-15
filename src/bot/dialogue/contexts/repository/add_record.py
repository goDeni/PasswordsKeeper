from typing import List

from aiogram.types import InlineKeyboardButton, InlineKeyboardMarkup, Message

from bot.dialogue.contexts.base import BaseContext
from bot.dialogue.contexts.commands import SHOW_COMMAND
from bot.dialogue.contexts.common import delete_messages
from dialog.types import CallbackName
from sec_store.record import Record

_CANCEL_ADD = CallbackName("_CANCEL_ADD")


class AddRecord(BaseContext[Record | None]):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._value_message: Message | None = None
        self._name_message: Message | None = None
        self._description_message: Message | None = None
        self._cancel_keyboard_message: Message | None = None

        self._sent_messages: List[Message] = []
        self.add_on_startup(self._send_enter_value_message)
        self.add_on_shutdown(self._delete_messages)

        self._commands_emitter.set_handler(SHOW_COMMAND, self._handle_show_command)

    async def _handle_show_command(self, message: Message):
        await delete_messages(message)

        if not self._sent_messages:
            self._set_result(None)
            return

        last_message = self._sent_messages.pop()

        await delete_messages(last_message, self._cancel_keyboard_message)
        await self._send_cancel_keyboard_message()
        self._sent_messages.append(await last_message.send_copy(self._user_id))

    async def _delete_messages(self):
        await delete_messages(*self._sent_messages, self._cancel_keyboard_message)

    async def _send_enter_value_message(self):
        await self._send_cancel_keyboard_message()
        self._sent_messages.append(
            await self._bot.send_message(
                self._user_id, "Введите значение для новой записи"
            )
        )

    async def _send_cancel_keyboard_message(self):
        keyboard_markup = InlineKeyboardMarkup().add(
            InlineKeyboardButton(
                text="Отменить добавление записи",
                callback_data=_CANCEL_ADD,
            )
        )
        self._callbacks_emitter.set_handler(_CANCEL_ADD, self._handle_cancel)
        self._cancel_keyboard_message = await self._bot.send_message(
            self._user_id, text="❌", reply_markup=keyboard_markup
        )

    async def _handle_cancel(self, unused_message: Message):
        self._set_result(None)

    async def _handle_message(self, message: Message):
        self._sent_messages.append(message)

        if self._value_message is None:
            self._value_message = message
            self._sent_messages.append(
                await self._bot.send_message(
                    self._user_id, "Введите название для новой записи"
                )
            )
            return

        if self._name_message is None:
            self._name_message = message
            self._sent_messages.append(
                await self._bot.send_message(
                    self._user_id, "Введите описание для новой записи"
                )
            )
            return

        self._description_message = message
        self._set_result(
            Record(
                name=self._name_message.text,
                description=self._description_message.text,
                value=self._value_message.text,
            )
        )
