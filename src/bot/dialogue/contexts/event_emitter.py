from asyncio import iscoroutine
from typing import Any, Callable, Coroutine, Dict, Generic, Tuple, TypeVar

_EventName = TypeVar("_EventName")
_EventObject = TypeVar("_EventObject")
_EventHandler = Callable[[_EventObject], Coroutine | Any]


class UnexpectedEventEmit(Exception):
    pass


class EventEmitter(Generic[_EventName, _EventObject]):
    def __init__(self) -> None:
        self.__handlers: Dict[_EventName, Tuple[_EventHandler, Tuple, Dict]] = {}

    def set_handler(
        self, name: _EventName, handler: _EventHandler, *handler_args, **handler_kwargs
    ):
        self.__handlers[name] = (handler, handler_args, handler_kwargs)

    def remove_all_handlers(self):
        self.__handlers.clear()

    async def emit(self, name: _EventName, obj: _EventObject):
        if name not in self.__handlers:
            raise UnexpectedEventEmit(
                f"Unexpected {name=}, excpected one of: {[*self.__handlers.keys()]}"
            )

        handler_fn, args, kwargs = self.__handlers[name]
        result = handler_fn(obj, *args, **kwargs)
        if iscoroutine(result):
            await result
