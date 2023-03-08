from asyncio import Lock
from collections import defaultdict
from contextlib import asynccontextmanager
from typing import Dict

_USERS_LOCKS: Dict[str, Lock] = defaultdict(Lock)
_USERS_WAITERS: Dict[str, int] = defaultdict(int)


@asynccontextmanager
async def lock_user_ctx(user_id: int):
    # Нужен для операций требующих того чтобы паралельно
    # не было обработки событий от этого же пользователя
    #
    # почему реализовано именно так?
    # т.к. в теории user_id может быть любым числом
    # то пытаемся избежать ситуации когда _USERS_LOCKS заполнен
    # Lock-ами которыми не пользуются в текущий момент времени
    #
    # p.s. пытаемся оптимизировать потребление ОЗУ
    lock = _USERS_LOCKS[user_id]
    _USERS_WAITERS[user_id] += 1

    async with lock:
        _USERS_WAITERS[user_id] -= 1
        yield

        if not _USERS_WAITERS[user_id]:
            _USERS_WAITERS.pop(user_id)
            _USERS_LOCKS.pop(user_id)
