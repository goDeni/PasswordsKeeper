from aiogram import Bot, Dispatcher

_DISPATCHER: Dispatcher | None = None
_BOT: Bot | None = None


def define_global_variables(
    *,
    dispatcher: Dispatcher,
    bot: Bot,
):
    global _DISPATCHER, _BOT

    if _DISPATCHER is not None:
        raise RuntimeError("_DISPATCHER already defined")

    if _BOT is not None:
        raise RuntimeError("_BOT already defined")

    _DISPATCHER = dispatcher
    _BOT = bot


def get_dispatcher() -> Dispatcher:
    if _DISPATCHER is None:
        raise RuntimeError("_DISPATCHER is not defined")

    return _DISPATCHER


def get_bot() -> Bot:
    if _BOT is None:
        raise RuntimeError("_BOT is not defined")

    return _BOT
