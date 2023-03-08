from pathlib import Path

import pytest

from sec_store.exceptions import WrongKeyHashError
from sec_store.key import KeyHash, hash_key
from sec_store.record import Record

_DATA_VALUES = [
    "Hello world",
    "Hello world with numbers 1234",
    "Hello world with numbers 1234 and very long string",
    "Hello world with numbers 1234 and very long string with special symbols [{\\}]'\",./?!@#@!$#)^%@#&*^%)*&#@%_",
]


@pytest.mark.parametrize("data", _DATA_VALUES)
def test_record_data_decryption(keyhash: KeyHash, data: str):
    record = Record(name="1", description="2", value=data)

    assert (
        Record.decode(encrypted_record=record.encrypt(keyhash), keyhash=keyhash)
        == record
    )


def test_record_access_with_wrong_keyhash(tmp_dir: Path, keyhash: KeyHash):
    encrypted = Record(name="1", description="2", value="Any data").encrypt(keyhash)

    with pytest.raises(WrongKeyHashError):
        Record.decode(encrypted, keyhash=hash_key("Wrong password"))
