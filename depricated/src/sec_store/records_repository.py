from json import dump, load
from pathlib import Path
from typing import Dict, List, NewType
from uuid import uuid4

from sec_store.cipher import (
    decode_text,
    dumps_encrypted_data,
    encode_text,
    loads_encrypted_data,
)
from sec_store.exceptions import RecordAlreadyExist, WrongKeyHashError
from sec_store.key import KeyHash
from sec_store.record import EncryptedRecord, Record, RecordId

_RepositoryId = NewType("_RepositoryId", str)


class RecordsRepository:
    def __init__(
        self,
        file: Path,
        records: Dict[RecordId, Record],
        keyhash: KeyHash,
        identifier: _RepositoryId,
    ) -> None:
        self._id = identifier
        self._record_by_id = records
        self._keyhash = keyhash
        self._transaction = False

        self._file = file

    @staticmethod
    def open(file: Path, keyhash: KeyHash) -> "RecordsRepository":
        with file.open(mode="r", encoding="utf-8") as file_fd:
            json_data = load(file_fd)

        try:
            repository_id: _RepositoryId = _RepositoryId(
                decode_text(loads_encrypted_data(json_data["id"]), keyhash)
            )
        except ValueError as err:
            raise WrongKeyHashError() from err

        encrypted_records: List[EncryptedRecord] = json_data["records"]

        return RecordsRepository(
            file=file,
            records={
                record.id: record
                for record in (
                    Record.decode(encrypted_record, keyhash)
                    for encrypted_record in encrypted_records
                )
            },
            identifier=repository_id,
            keyhash=keyhash,
        )

    @staticmethod
    def new(file: Path, keyhash: KeyHash) -> "RecordsRepository":
        return RecordsRepository(
            file=file,
            records={},
            keyhash=keyhash,
            identifier=_RepositoryId(uuid4().hex),
        )

    def save(self) -> None:
        with self._file.open(mode="w", encoding="utf-8") as file_fd:
            dump(
                {
                    "id": dumps_encrypted_data(encode_text(self._id, self._keyhash)),
                    "records": [
                        record.encrypt(self._keyhash)
                        for record in self._record_by_id.values()
                    ],
                },
                file_fd,
            )

    @property
    def records(self) -> List[Record]:
        return [*self._record_by_id.values()]

    def get(self, record_id: RecordId) -> Record:
        return self._record_by_id[record_id]

    def delete(self, record_id: RecordId):
        del self._record_by_id[record_id]

    def add_record(self, record: Record) -> None:
        if record.id in self._record_by_id:
            raise RecordAlreadyExist

        self._record_by_id[record.id] = record
