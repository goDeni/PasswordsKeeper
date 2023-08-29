from asyncio import create_task, iscoroutine
from typing import Callable, Coroutine, List


async def _call(functions: List[Callable[[], Coroutine | None]]):
    for func in functions:
        result = func()
        if iscoroutine(result):
            await result


_OnStartupShutdownFn = Callable[[], Coroutine | None]


class OnStartupShutdown:
    def __init__(self) -> None:
        self.__called = False

        self.__on_startup: List[_OnStartupShutdownFn] = []
        self.__on_shutdown: List[_OnStartupShutdownFn] = []

        create_task(_call(self.__on_startup), name=f"_on_startup {type(self)}")

    def add_on_startup(self, *functions: _OnStartupShutdownFn):
        self.__on_startup.extend(functions)

    def add_on_shutdown(self, *functions: _OnStartupShutdownFn):
        self.__on_shutdown.extend(functions)

    async def shutdown(self):
        if self.__called:
            raise RuntimeError("shutdown already called")
        self.__called = True

        await _call(self.__on_shutdown)
