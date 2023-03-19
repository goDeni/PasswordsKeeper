from asyncio import create_task, sleep
from logging import getLogger
from time import monotonic

from aiogram import Bot
from aiogram.types import Message

from bot.dialogue.contexts.common import delete_messages
from dialog.context import (
    CallbackName,
    Command,
    Context,
    CtxResultType,
    UnexpectedCallback,
    UnexpectedCommand,
)

logger = getLogger(__name__)


class BaseContext(Context[Message, CtxResultType]):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__()

        self._bot = bot
        self._user_id = user_id

    def _kill_ctx_if_unused(self, seconds: int, exit_reason: str):
        async def _watch_for_time_without_use():
            while (delta := (self._last_usage + seconds) - monotonic()) > 0:
                await sleep(delta)

            if self.ctx_is_over:
                return

            self._exit_from_ctx()
            await self._bot.send_message(self._user_id, exit_reason)

        task = create_task(
            _watch_for_time_without_use(),
            name=f"_watch_for_time_without_use {self._user_id}",
        )
        self.add_on_shutdown(task.cancel)

    async def _handle_message(self, message: Message):
        await delete_messages(message)

    async def _handle_callback(self, callback: CallbackName, message: Message):
        try:
            await super()._handle_callback(callback, message)
        except UnexpectedCallback:
            logger.warning(
                "Handled unexpected callback '%s', user_id=%s type=%s",
                callback,
                self._user_id,
                type(self),
            )
            await delete_messages(message)

    async def _handle_command(self, command: Command, message: Message):
        try:
            await super()._handle_command(command, message)
        except UnexpectedCommand:
            logger.warning(
                "Handled unexpected command '%s', user_id=%s type=%s",
                command,
                self._user_id,
                type(self),
            )
            await delete_messages(message)
