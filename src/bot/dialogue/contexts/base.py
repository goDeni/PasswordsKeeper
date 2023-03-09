from asyncio import create_task
from functools import partial
from logging import getLogger
from typing import (
    Callable,
    Coroutine,
    Dict,
    Generic,
    List,
    NewType,
    Optional,
    Type,
    TypeVar,
    final,
)

from aiogram import Bot
from aiogram.types import CallbackQuery, Message

logger = getLogger(__name__)
CallbackName = NewType("CallbackName", str)


class _CallbackHandling:
    def __init__(self) -> None:
        self.__callbacks: Dict[CallbackName, Callable[[CallbackQuery], Coroutine]] = {}

    @final
    async def _handle_callback(
        self, callback: CallbackName, callback_query: CallbackQuery
    ):
        callback_fn = self.__callbacks.get(callback, None)
        if callback_fn is None:
            logger.warning(
                "Handled unexpected callback '%s' type=%s",
                callback,
                type(self),
            )
            await callback_query.message.delete()
            return

        await callback_fn(callback_query=callback_query)

    def _set_callback(
        self,
        callback: CallbackName,
        callback_fn: Callable[[CallbackQuery], Coroutine],
    ):
        self.__callbacks[callback] = callback_fn


class _MessageHandling:
    async def _handle_message(self, message: Message):
        await message.delete()


class _OnStartup:
    def __init__(self) -> None:
        self._on_startup: List[Coroutine] = []
        create_task(self.__on_startup(), name=f"_on_startup {type(self)}")

    async def __on_startup(self):
        for startup_coro in self._on_startup:
            await startup_coro


_CtxResultType = TypeVar("_CtxResultType")  # pylint: disable=invalid-name
_SubCtxResultType = TypeVar("_SubCtxResultType")  # pylint: disable=invalid-name
_Default = object()
_Exit = object()


class BaseContext(
    Generic[_CtxResultType], _CallbackHandling, _MessageHandling, _OnStartup
):
    def __init__(self, bot: Bot, user_id: int) -> None:
        _MessageHandling.__init__(self)
        _CallbackHandling.__init__(self)
        _OnStartup.__init__(self)

        self._bot = bot
        self._user_id = user_id

        self.__new_ctx: Callable[[], "BaseContext"] = None
        self.__sub_ctx: BaseContext[_SubCtxResultType] | None = None
        self.__result: _CtxResultType | _Default | _Exit = _Default

    @final
    async def handle_message(self, *args, **kwargs):
        instance = self if self.__sub_ctx is None else self.__sub_ctx
        await instance._handle_message(  # pylint: disable=protected-access
            *args, **kwargs
        )
        await self.__remove_sub_ctx_if_needed()

    @final
    async def handle_callback(self, *args, **kwargs):
        instance = self if self.__sub_ctx is None else self.__sub_ctx
        await instance._handle_callback(  # pylint: disable=protected-access
            *args, **kwargs
        )
        await self.__remove_sub_ctx_if_needed()

    async def _handle_sub_ctx_result(
        self, sub_ctx: "BaseContext[_SubCtxResultType]"
    ):  # pylint: disable=unused-argument
        pass

    def _set_sub_ctx(self, sub_ctx: "BaseContext[_SubCtxResultType]"):
        self.__sub_ctx = sub_ctx

    def _set_result(self, result: _CtxResultType):
        self.__result = result

    def _exit_from_ctx(self):
        self.__result = _Exit

    @property
    def result(self) -> _CtxResultType:
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

        await self._handle_sub_ctx_result(sub_ctx)

    @final
    def get_new_ctx(self) -> Optional["BaseContext"]:
        if self.__new_ctx is None:
            return None

        return self.__new_ctx()

    @final
    def _set_new_ctx(self, next_ctx_class: Type["BaseContext"], *args, **kwargs):
        if self.ctx_is_over:
            raise RuntimeError(f"{self.__ctx_is_over=} can't set next ctx")

        if self.__new_ctx is not None:
            raise RuntimeError(f"{self.__new_ctx=} already is not None")

        self.__new_ctx = partial(
            next_ctx_class, self._bot, self._user_id, *args, **kwargs
        )
