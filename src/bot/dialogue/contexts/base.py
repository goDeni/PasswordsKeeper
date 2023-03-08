from functools import partial
from logging import getLogger
from typing import (
    Callable,
    Coroutine,
    Dict,
    Generic,
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


class _CtxIsOver:
    def __init__(self) -> None:
        self.__ctx_is_over = False

    @property
    def ctx_is_over(self) -> bool:
        return self.__ctx_is_over

    def _set_ctx_over(self):
        self.__ctx_is_over = True


_SubCtxResultType = TypeVar("_SubCtxResultType")  # pylint: disable=invalid-name
_Default = object()


class BaseSubContext(
    Generic[_SubCtxResultType], _MessageHandling, _CallbackHandling, _CtxIsOver
):
    def __init__(self, bot: Bot, user_id: int) -> None:
        _MessageHandling.__init__(self)
        _CallbackHandling.__init__(self)
        _CtxIsOver.__init__(self)

        self._bot = bot
        self._user_id = user_id

        self._sub_ctx_over = False
        self._result: _SubCtxResultType | _Default = _Default

    def _set_result(self, result: _SubCtxResultType):
        self._result = result
        self._set_ctx_over()

    @property
    def result(self) -> _SubCtxResultType:
        if self._result == _Default:
            raise RuntimeError("self._result is default!")

        return self._result


class BaseContext(_CallbackHandling, _MessageHandling, _CtxIsOver):
    def __init__(self, bot: Bot, user_id: int) -> None:
        _MessageHandling.__init__(self)
        _CallbackHandling.__init__(self)
        _CtxIsOver.__init__(self)

        self._bot = bot
        self._user_id = user_id

        self.__new_ctx: Callable[[], "BaseContext"] = None
        self.__sub_ctx: BaseSubContext[_SubCtxResultType] | None = None

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
        self, sub_ctx: _SubCtxResultType
    ):  # pylint: disable=unused-argument
        logger.warning(
            "Called not overridden method %s type=%s",
            "_handle_sub_ctx_result",
            type(self),
        )

    def _set_sub_ctx(self, sub_ctx: BaseSubContext):
        self.__sub_ctx = sub_ctx

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
