from aiogram.types import (
    CallbackQuery,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    Message,
)

from bot.dialogue.contexts.base import BaseContext, CallbackName
from bot.dialogue.contexts.commands import SHOW_COMMAND
from bot.dialogue.contexts.common import delete_messages
from bot.dialogue.contexts.repository.add_record import AddRecord
from bot.dialogue.contexts.repository.edit_record import EditRecord, EditResult
from bot.dialogue.contexts.repository.view_record import RecordAction, ViewRecord
from sec_store.record import Record
from sec_store.records_repository import RecordsRepository

_VIEW_RECORD_CALLBACK = CallbackName("_VIEW_RECORD_CALLBACK")
_ADD_RECORD_CALLBACK = CallbackName("_ADD_RECORD_CALLBACK")
_CLOSE_REPOSITORY = CallbackName("_CLOSE_REPOSITORY")

_MAX_TIME_WITHOUT_USE = 60 * 10


class RepositoryViewCtx(BaseContext[None]):
    def __init__(self, *args, records_repository: RecordsRepository, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._records_repository = records_repository

        self._records_view_message: Message | None = None

        self._on_startup.append(self._send_records_keyboard())
        self._on_shutdown.append(self._delete_messages())

        self._commands_emitter.set_handler(SHOW_COMMAND, self._handle_show_command)
        self._kill_ctx_if_unused(
            _MAX_TIME_WITHOUT_USE,
            f"Репозиторий закрыт т.к. не используется в течении {_MAX_TIME_WITHOUT_USE} секунд",
        )

    async def _delete_messages(self):
        await delete_messages(self._records_view_message)

    async def _handle_show_command(self, message: Message):
        await delete_messages(message)
        await self._send_records_keyboard()

    async def _send_records_keyboard(self):
        self._callbacks_emitter.remove_all_handlers()

        keyboard_markup = InlineKeyboardMarkup()
        for record in self._records_repository.records:
            keyboard_markup.row(
                InlineKeyboardButton(
                    f"{record.name}",
                    callback_data=f"{_VIEW_RECORD_CALLBACK}-{record.id}",
                )
            )
            self._callbacks_emitter.set_handler(
                f"{_VIEW_RECORD_CALLBACK}-{record.id}",
                self._view_record_callback,
                record,
            )

        keyboard_markup.row(
            InlineKeyboardButton(
                "🗒 Добавить запись",
                callback_data=_ADD_RECORD_CALLBACK,
            )
        ).row(
            InlineKeyboardButton(
                "🚪 Закрыть репозиторий",
                callback_data=_CLOSE_REPOSITORY,
            )
        )
        self._callbacks_emitter.set_handler(
            _CLOSE_REPOSITORY, self._close_repository_callback
        )
        self._callbacks_emitter.set_handler(
            _ADD_RECORD_CALLBACK, self._add_record_callback
        )

        await delete_messages(self._records_view_message)
        self._records_view_message = await self._bot.send_message(
            self._user_id,
            f"Кол-во: {len(self._records_repository.records)}",
            reply_markup=keyboard_markup,
        )

    async def _handle_sub_ctx_result(
        self, sub_ctx: AddRecord | ViewRecord | EditRecord
    ):
        match sub_ctx:
            case AddRecord():
                if isinstance(sub_ctx.result, Record):
                    self._records_repository.add_record(sub_ctx.result)
                    self._records_repository.save()
            case EditRecord():
                if sub_ctx.result == EditResult.SAVE:
                    self._records_repository.save()
            case ViewRecord():
                match sub_ctx.result:
                    case (RecordAction.EDIT, record_id):
                        self._set_sub_ctx(
                            EditRecord(
                                self._bot,
                                self._user_id,
                                record=self._records_repository.get(record_id),
                            )
                        )
                        return
                    case (RecordAction.DELETE, record_id):
                        self._records_repository.delete(record_id)
                        self._records_repository.save()

        await self._send_records_keyboard()

    async def _view_record_callback(
        self,
        callback_query: CallbackQuery,
        record: Record,
    ):
        self._set_sub_ctx(ViewRecord(self._bot, self._user_id, record=record))
        await delete_messages(callback_query.message)

    async def _close_repository_callback(self, callback_query: CallbackQuery):
        self._exit_from_ctx()
        await delete_messages(callback_query.message)

    async def _add_record_callback(self, callback_query: CallbackQuery):
        self._set_sub_ctx(AddRecord(self._bot, self._user_id))
        await delete_messages(callback_query.message)
