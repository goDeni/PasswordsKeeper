from asyncio import create_task

from aiogram import Bot
from aiogram.types import Message

from bot.dialogue.contexts.base import BaseContext
from bot.dialogue.contexts.repository.rep_view import RepositoryViewCtx
from bot.user_reps import get_user_repository
from sec_store.exceptions import WrongKeyHashError
from sec_store.key import hash_key


class OpenRepositoryCtx(BaseContext):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__(bot, user_id)

        self._enter_password_message: Message | None = None
        create_task(
            self._send_enter_password_message(),
            name=f"Send enter password message {user_id=}",
        )

    async def _handle_sub_ctx_result(self, sub_ctx: None):
        raise NotImplementedError

    async def _send_enter_password_message(self):
        self._enter_password_message = await self._bot.send_message(
            self._user_id, "Введите пароль"
        )

    async def _handle_message(self, message: Message):
        await message.delete()
        if self._enter_password_message is None:
            return

        keyhash = hash_key(message.text)

        await self._enter_password_message.delete()
        try:
            repository = get_user_repository(self._user_id, keyhash)
        except WrongKeyHashError:
            self._enter_password_message = await self._bot.send_message(
                self._user_id, "Введен не верный пароль. Введите еще раз"
            )
            return

        self._set_new_ctx(RepositoryViewCtx, records_repository=repository)
