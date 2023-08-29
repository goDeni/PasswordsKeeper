from dataclasses import asdict, dataclass, field
from json import dumps, loads
from typing import NewType
from uuid import uuid4

from sec_store.cipher import (
    check_keyhash,
    decode_text,
    dumps_encrypted_data,
    encode_text,
    loads_encrypted_data,
)
from sec_store.exceptions import WrongKeyHashError
from sec_store.key import KeyHash

RecordId = NewType("RecordId", str)
EncryptedRecord = NewType("EncryptedRecord", str)


def _new_record_id() -> RecordId:
    return RecordId(str(uuid4()))


@dataclass
class Record:
    name: str
    description: str
    value: str

    id: RecordId = field(  # pylint: disable=invalid-name
        default_factory=_new_record_id, kw_only=True
    )

    def encrypt(self, keyhash: KeyHash) -> EncryptedRecord:
        return EncryptedRecord(
            dumps_encrypted_data(
                encrypted_data=encode_text(
                    text=dumps(asdict(self)),
                    keyhash=keyhash,
                ),
            )
        )

    @staticmethod
    def decode(encrypted_record: EncryptedRecord, keyhash: KeyHash) -> "Record":
        encrypted_data = loads_encrypted_data(encrypted_record)

        try:
            raw_record = decode_text(
                encrypted_data=encrypted_data,
                keyhash=check_keyhash(keyhash),
            )
        except ValueError as err:
            raise WrongKeyHashError() from err

        return Record(**loads(raw_record))
