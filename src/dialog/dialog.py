from asyncio import Event, Lock
from asyncio import TimeoutError as AsyncTimeoutError
from asyncio import create_task, wait_for
from logging import getLogger
from typing import Callable, Coroutine, Generic

from dialog.context import Context
from dialog.types import CallbackName, Command, Message

logger = getLogger(__name__)


class Dialog(Generic[Message]):
    def __init__(
        self,
        get_root_ctx_fn: Callable[[], Context],
    ) -> None:
        self.__ctx_lock = Lock()
        self.__check_ctx_change = Event()

        self.__get_root_ctx = get_root_ctx_fn
        self.__ctx: Context[Message] = self.__get_root_ctx()

        self.__watch_task = create_task(
            _call_on_event_or_interval(
                self.__check_ctx_change, self._switch_ctx_if_needed, 1
            ),
            name="Watch ctx changing",
        )

    async def shutdown(self):
        async with self.__ctx_lock:
            self.__watch_task.cancel()
            await self.__ctx.shutdown()

    async def handle_command(self, command: Command, message: Message):
        async with self.__ctx_lock:
            await self.__ctx.handle_command(command, message)
            self.__check_ctx_change.set()

    async def handle_message(self, message: Message):
        async with self.__ctx_lock:
            await self.__ctx.handle_message(message)
            self.__check_ctx_change.set()

    async def handle_callback(self, callback: CallbackName, message: Message):
        async with self.__ctx_lock:
            await self.__ctx.handle_callback(callback, message)
            self.__check_ctx_change.set()

    async def _change_ctx(self, new_ctx: Context):
        old_ctx = self.__ctx
        self.__ctx = new_ctx

        await old_ctx.shutdown()

    async def _switch_ctx_if_needed(self):
        async with self.__ctx_lock:
            if self.__ctx.ctx_is_over:
                await self._change_ctx(self.__get_root_ctx())
                return

            new_ctx = self.__ctx.get_new_ctx()
            if new_ctx is None:
                return

            await self._change_ctx(new_ctx)


async def _call_on_event_or_interval(
    event: Event, func: Callable[[], Coroutine], interval: float
):
    while True:
        try:
            await wait_for(event.wait(), timeout=interval)
            event.clear()
        except AsyncTimeoutError:
            pass
        except Exception:  # pylint: disable=broad-except
            logger.exception("Unexpected error")
            continue

        await func()
