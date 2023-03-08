from asyncio import create_task

from aiogram import Bot
from aiogram.types import (
    CallbackQuery,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    Message,
)

from bot.dialogue.contexts.base import BaseContext, CallbackName
from bot.user_reps import initialize_user_repository
from sec_store.key import hash_key

_CREATE_REPOSITORY_CALLBACK = CallbackName("initialize_repository")
_CANCEL_REPOSITORY_INITIALIZATION = CallbackName("cancel_repo_initialization")


class InitializeRepCtx(BaseContext):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__(bot, user_id)

        self._password_input: _PasswordInput | None = None
        self._rep_creation_message: Message | None = None

        create_task(
            self._send_init_keyboard(),
            name=f"Send initialize rep keyboard for {user_id=}",
        )

    async def _handle_sub_ctx_result(self, sub_ctx: None):
        raise NotImplementedError

    async def _handle_message(self, message: Message):
        if self._password_input is None:
            await message.delete()
            return

        await self._password_input.handle_message(message)
        result_password = self._password_input.result_password
        if result_password is None:
            return

        initialize_user_repository(self._user_id, hash_key(result_password))
        await self._bot.send_message(self._user_id, "Репозиторий успешно создан!")
        if self._rep_creation_message is not None:
            await self._rep_creation_message.delete()

        self._set_ctx_over()

    async def _cancel_repository_creation_callback(self, callback_query: CallbackQuery):
        self._set_ctx_over()
        await callback_query.message.edit_text(
            "Создание репозитория отменено", reply_markup=None
        )

    async def _create_repositiry_callback(self, callback_query: CallbackQuery):
        keyboard_markup = InlineKeyboardMarkup(row_width=3)
        keyboard_markup.row(
            InlineKeyboardButton(
                "Отменить создание репозитория",
                callback_data=_CANCEL_REPOSITORY_INITIALIZATION,
            )
        )

        self._rep_creation_message = await callback_query.message.edit_text(
            "Создание репозитория",
            reply_markup=keyboard_markup,
        )
        self._set_callback(
            _CANCEL_REPOSITORY_INITIALIZATION, self._cancel_repository_creation_callback
        )
        self._password_input = _PasswordInput(self._bot, self._user_id)

    async def _send_init_keyboard(self):
        keyboard_markup = InlineKeyboardMarkup(row_width=3)
        keyboard_markup.row(
            InlineKeyboardButton(
                "Создать репозиторий",
                callback_data=_CREATE_REPOSITORY_CALLBACK,
            )
        )

        await self._bot.send_message(
            self._user_id, "Выберите действие", reply_markup=keyboard_markup
        )

        self._set_callback(
            _CREATE_REPOSITORY_CALLBACK, self._create_repositiry_callback
        )


class _PasswordInput:
    def __init__(self, bot: Bot, user_id: str) -> None:
        self._bot = bot
        self._user_id = user_id

        create_task(
            self._send_hello_message(), name=f"Password enter message {user_id=}"
        )

        self._enter_password_message: Message | None = None

        self._password_1 = None
        self._password_2 = None

    @property
    def result_password(self) -> str | None:
        if self._password_1 == self._password_2:
            return self._password_1

        return None

    async def _send_hello_message(self):
        self._enter_password_message = await self._bot.send_message(
            self._user_id, "Придумайте пароль"
        )

    async def handle_message(self, message: Message):
        await message.delete()
        await self._enter_password_message.delete()

        if self._password_1 is None:
            self._password_1 = message.text
            self._enter_password_message = await self._bot.send_message(
                self._user_id, "Повторите пароль"
            )
            return

        self._password_2 = message.text
        if self._password_1 != self._password_2:
            self._enter_password_message = await self._bot.send_message(
                self._user_id, "Пароли не совпадают. Попробуйте еще раз"
            )
