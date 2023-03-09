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

_CREATE_REPOSITORY_CALLBACK = CallbackName("_CREATE_REPOSITORY_CALLBACK")
_CANCEL_PASSWORD_INPUT = CallbackName("_CANCEL_PASSWORD_INPUT")


class InitializeRepCtx(BaseContext[None]):
    def __init__(self, bot: Bot, user_id: int) -> None:
        super().__init__(bot, user_id)
        self._password_input: _PasswordInput | None = None

        self._on_startup.append(self._send_init_keyboard())

    async def _handle_sub_ctx_result(self, sub_ctx: "_PasswordInput"):
        if sub_ctx.result is not None:
            initialize_user_repository(self._user_id, hash_key(sub_ctx.result))
            await self._bot.send_message(self._user_id, "Репозиторий успешно создан!")

        self._set_result(None)

    async def _handle_message(self, message: Message):
        await message.delete()

    async def _create_repositiry_callback(self, callback_query: CallbackQuery):
        self._set_sub_ctx(_PasswordInput(self._bot, self._user_id))
        await callback_query.message.delete()

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


class _PasswordInput(BaseContext[str | None]):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._password_creation_message: Message | None = None
        self._enter_password_message: Message | None = None

        self._password_1 = None
        self._password_2 = None

        self._on_startup.append(self._send_hello_message())

    async def _send_hello_message(self):
        self._password_creation_message = await self._bot.send_message(
            self._user_id,
            "Создание пароля",
            reply_markup=InlineKeyboardMarkup().row(
                InlineKeyboardButton(
                    "Отменить создание пароля",
                    callback_data=_CANCEL_PASSWORD_INPUT,
                )
            ),
        )
        self._set_callback(_CANCEL_PASSWORD_INPUT, self._cancel_password_input)
        self._enter_password_message = await self._bot.send_message(
            self._user_id, "Придумайте пароль"
        )

    async def _cancel_password_input(self, callback_query: CallbackQuery):
        self._set_result(None)

        if self._enter_password_message is not None:
            await self._enter_password_message.delete()
        await callback_query.message.delete()

    async def _handle_message(self, message: Message):
        await message.delete()
        if self._enter_password_message is not None:
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
            return

        if self._password_creation_message is not None:
            await self._password_creation_message.delete()

        self._set_result(self._password_1)
