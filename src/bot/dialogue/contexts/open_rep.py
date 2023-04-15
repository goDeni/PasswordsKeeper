from aiogram import Bot
from aiogram.types import Message

from bot.dialogue.contexts.base import BaseContext
from bot.dialogue.contexts.common import delete_messages
from bot.dialogue.contexts.repository.rep_view import RepositoryViewCtx
from bot.user_reps import get_user_repository
from sec_store.exceptions import WrongKeyHashError
from sec_store.key import hash_key


class OpenRepositoryCtx(BaseContext):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__(bot, user_id)

        self._enter_password_message: Message | None = None
        self.add_on_startup(self._send_enter_password_message)

    async def _send_enter_password_message(self):
        self._enter_password_message = await self._bot.send_message(
            self._user_id, "Введите пароль"
        )

    async def _handle_message(self, message: Message):
        await delete_messages(message, self._enter_password_message)

        if self._enter_password_message is None:
            return

        self._enter_password_message = None
        keyhash = hash_key(message.text)
        try:
            repository = get_user_repository(self._user_id, keyhash)
        except WrongKeyHashError:
            self._enter_password_message = await self._bot.send_message(
                self._user_id, "Введен не верный пароль. Введите еще раз"
            )
            return

        self._set_new_ctx(RepositoryViewCtx, self._bot, self._user_id, repository)
