from asyncio import Event, Lock
from asyncio import TimeoutError as AsyncTimeoutError
from asyncio import create_task, wait_for
from logging import getLogger
from typing import Type

from aiogram import Bot
from aiogram.types import CallbackQuery, Message

from bot.dialogue.contexts.base import BaseContext

logger = getLogger(__name__)


class UserDialog:
    def __init__(
        self,
        bot: Bot,
        default_ctx_class: Type[BaseContext],
        user_id: int,
    ) -> None:
        self._bot = bot

        self._lock = Lock()
        self._user_id = user_id
        self._default_ctx_class = default_ctx_class
        self._check_ctx_change = Event()

        self._ctx: BaseContext = default_ctx_class(bot, self._user_id)

        self._watch_task = create_task(
            self._watch_ctx_changing(), name=f"Watch ctx changing {user_id=}"
        )

    async def shutdown(self):
        async with self._lock:
            self._watch_task.cancel()
            await self._ctx.shutdown()

    async def handle_command(self, message: Message):
        async with self._lock:
            await self._ctx.handle_command(message)
            self._check_ctx_change.set()

    async def handle_message(self, message: Message):
        async with self._lock:
            await self._ctx.handle_message(message)
            self._check_ctx_change.set()

    async def handle_callback(self, query: CallbackQuery):
        async with self._lock:
            await self._ctx.handle_callback(query)
            self._check_ctx_change.set()

    async def _watch_ctx_changing(self):
        while True:
            try:
                await wait_for(self._check_ctx_change.wait(), timeout=1)
                self._check_ctx_change.clear()
            except AsyncTimeoutError:
                pass
            except Exception:  # pylint: disable=broad-except
                logger.exception(
                    "Unexpected error during watching context changes: user_id=%s",
                    self._user_id,
                )
                continue

            await self._switch_ctx_if_needed()

    async def _set_new_ctx(self, new_ctx: BaseContext):
        old_ctx = self._ctx
        self._ctx = new_ctx

        await old_ctx.shutdown()

    async def _switch_ctx_if_needed(self):
        if self._ctx.ctx_is_over:
            await self._set_new_ctx(self._default_ctx_class(self._bot, self._user_id))
            return

        new_ctx = self._ctx.get_new_ctx()
        if new_ctx is None:
            return

        await self._set_new_ctx(new_ctx)
