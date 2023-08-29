from asyncio import gather
from typing import Optional

from aiogram.types import Message
from aiogram.utils.exceptions import MessageToDeleteNotFound


async def delete_messages(*messages: Optional[Message]):
    async def _del_message(msg: Message):
        try:
            await msg.delete()
        except MessageToDeleteNotFound:
            pass

    await gather(
        *[_del_message(message) for message in messages if message is not None]
    )
