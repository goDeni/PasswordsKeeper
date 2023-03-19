from functools import partial
from logging import getLogger
from time import monotonic
from typing import Callable, Generic, Optional, TypeVar, final


from dialog.event_emitter import EventEmitter, UnexpectedEventEmit
from dialog.on_startup_shutdown import OnStartupShutdown
from dialog.types import CallbackName, Command, Message

logger = getLogger(__name__)

CtxResultType = TypeVar("CtxResultType")  # pylint: disable=invalid-name


_SubCtxResultType = TypeVar("_SubCtxResultType")  # pylint: disable=invalid-name
_Default = object()
_Exit = object()


class UnexpectedCommand(Exception):
    pass


class UnexpectedCallback(Exception):
    pass


class Context(
    Generic[Message, CtxResultType], OnStartupShutdown
):  # pylint: disable=too-many-instance-attributes
    def __init__(self) -> None:
        OnStartupShutdown.__init__(self)

        self._callbacks_emitter: EventEmitter[CallbackName, Message] = EventEmitter()
        self._commands_emitter: EventEmitter[Command, Message] = EventEmitter()

        self._last_usage = monotonic()
        self.__new_ctx_fn: Callable[[], "Context"] = None
        self.__sub_ctx: Context[_SubCtxResultType] | None = None
        self.__result: CtxResultType | _Default | _Exit = _Default

        self.add_on_shutdown(self.__shutdown_sub_ctx_if_exist)

    async def __shutdown_sub_ctx_if_exist(self):
        if self.__sub_ctx is None:
            return

        await self.__sub_ctx.shutdown()

    async def _handle_message(self, message: Message):
        raise NotImplementedError

    async def _handle_callback(self, callback: CallbackName, message: Message):
        try:
            await self._callbacks_emitter.emit(callback, message)
        except UnexpectedEventEmit:
            raise UnexpectedCallback from None

    async def _handle_command(self, command: Command, message: Message):
        try:
            await self._commands_emitter.emit(command, message)
        except UnexpectedEventEmit:
            raise UnexpectedCommand from None

    async def _handle_sub_ctx_result(
        self, sub_ctx: "Context[_SubCtxResultType]"
    ):  # pylint: disable=unused-argument
        pass

    @final
    async def handle_message(self, message: Message):
        self._last_usage = monotonic()
        if self.__sub_ctx is None:
            await self._handle_message(message)
        else:
            await self.__sub_ctx.handle_message(message)
        await self.__remove_sub_ctx_if_needed()

    @final
    async def handle_callback(self, callback: CallbackName, message: Message):
        self._last_usage = monotonic()
        if self.__sub_ctx is None:
            await self._handle_callback(callback, message)
        else:
            await self.__sub_ctx.handle_callback(callback, message)
        await self.__remove_sub_ctx_if_needed()

    @final
    async def handle_command(self, command: Command, message: Message):
        self._last_usage = monotonic()
        if self.__sub_ctx is None:
            await self._handle_command(command, message)
        else:
            await self.__sub_ctx.handle_command(command, message)
        await self.__remove_sub_ctx_if_needed()

    def _set_sub_ctx(self, sub_ctx: "Context[_SubCtxResultType]"):
        self.__sub_ctx = sub_ctx

    def _set_result(self, result: CtxResultType):
        self.__result = result

    def _exit_from_ctx(self):
        self.__result = _Exit

    @property
    def result(self) -> CtxResultType:
        if self.__result == _Default:
            raise RuntimeError("self._result is default!")

        return self.__result

    @property
    def ctx_is_over(self) -> bool:
        return self.__result is not _Default

    async def __remove_sub_ctx_if_needed(self):
        if self.__sub_ctx is None:
            return

        if not self.__sub_ctx.ctx_is_over:
            return

        sub_ctx = self.__sub_ctx
        self.__sub_ctx = None

        await sub_ctx.shutdown()
        await self._handle_sub_ctx_result(sub_ctx)

    @final
    def get_new_ctx(self) -> Optional["Context"]:
        if self.__new_ctx_fn is None:
            return None

        return self.__new_ctx_fn()

    @final
    def _set_new_ctx(self, new_ctx_fn: Callable[[], "Context"], *args, **kwargs):
        if self.ctx_is_over:
            raise RuntimeError("Can't set new ctx: ctx is over")

        if self.__new_ctx_fn is not None:
            raise RuntimeError(f"{self.__new_ctx_fn=} already defined")

        self.__new_ctx_fn = partial(new_ctx_fn, *args, **kwargs)
