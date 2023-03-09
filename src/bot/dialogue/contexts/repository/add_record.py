from asyncio import gather
from typing import List

from aiogram.types import Message

from bot.dialogue.contexts.base import BaseSubContext
from sec_store.record import Record


class AddRecord(BaseSubContext[Record]):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._value_message: Message | None = None
        self._name_message: Message | None = None
        self._description_message: Message | None = None

        self._sent_messages: List[Message] = []
        self._on_startup.append(self._send_enter_value_message())

    async def _send_enter_value_message(self):
        self._sent_messages.append(
            await self._bot.send_message(self._user_id, "Введите значение")
        )

    async def _handle_message(self, message: Message):
        if self._value_message is None:
            self._value_message = message
            self._sent_messages.append(
                await self._bot.send_message(self._user_id, "Введите название")
            )
            return

        if self._name_message is None:
            self._name_message = message
            self._sent_messages.append(
                await self._bot.send_message(self._user_id, "Введите описание")
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
        await gather(
            *[msg.delete() for msg in self._sent_messages],
            self._value_message.delete(),
            self._name_message.delete(),
            self._description_message.delete(),
        )
