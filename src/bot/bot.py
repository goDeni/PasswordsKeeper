import logging

from aiogram import Bot, Dispatcher, executor

from bot.common import define_global_variables
from bot.config import get_bot_token

# Configure logging
logging.basicConfig(level=logging.INFO)


def main():
    bot = Bot(token=get_bot_token())
    dispatcher = Dispatcher(bot)

    define_global_variables(
        dispatcher=dispatcher,
        bot=bot,
    )

    import bot.handlers  # pylint: disable=import-outside-toplevel

    executor.start_polling(dispatcher, skip_updates=True)


if __name__ == "__main__":
    main()
