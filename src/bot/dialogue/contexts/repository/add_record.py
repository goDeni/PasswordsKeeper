from typing import List

from aiogram.types import Message

from bot.dialogue.contexts.base import BaseContext
from bot.dialogue.contexts.commands import SHOW_COMMAND
from bot.dialogue.contexts.common import delete_messages
from sec_store.record import Record


class AddRecord(BaseContext[Record | None]):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._value_message: Message | None = None
        self._name_message: Message | None = None
        self._description_message: Message | None = None

        self._sent_messages: List[Message] = []
        self._on_startup.append(self._send_enter_value_message())
        self._on_shutdown.append(self._delete_messages())

        self._commands_emitter.set_handler(SHOW_COMMAND, self._handle_show_command)

    async def _handle_show_command(self, message: Message):
        await delete_messages(message)

        if not self._sent_messages:
            self._set_result(None)
            return

        last_message = self._sent_messages.pop()

        await delete_messages(last_message)
        self._sent_messages.append(await last_message.send_copy(self._user_id))

    async def _delete_messages(self):
        await delete_messages(*self._sent_messages)

    async def _send_enter_value_message(self):
        self._sent_messages.append(
            await self._bot.send_message(
                self._user_id, "Введите значение для новой записи"
            )
        )

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
