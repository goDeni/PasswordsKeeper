import json
import os
import tempfile
from pathlib import Path
from typing import TypedDict

from Crypto.Cipher import AES

from sec_store.key import KeyHash, check_keyhash


class EncryptedData(TypedDict):
    data: bytes
    tag: bytes
    nonce: bytes


def encode_text(text: str, keyhash: KeyHash) -> EncryptedData:
    # https://pycryptodome.readthedocs.io/en/latest/src/cipher/modern.html#eax-mode-1
    cipher = AES.new(
        key=check_keyhash(keyhash),
        mode=AES.MODE_EAX,
    )

    encrypted_data, tag = cipher.encrypt_and_digest(text.encode("utf-8"))  # type: ignore
    return EncryptedData(
        data=encrypted_data,
        tag=tag,
        nonce=cipher.nonce,  # type: ignore
    )


def decode_text(encrypted_data: EncryptedData, keyhash: KeyHash) -> str:
    # https://pycryptodome.readthedocs.io/en/latest/src/cipher/modern.html#eax-mode-1
    return (
        AES.new(  # type: ignore
            key=check_keyhash(keyhash),
            mode=AES.MODE_EAX,
            nonce=encrypted_data["nonce"],
        )
        .decrypt_and_verify(
            encrypted_data["data"],
            encrypted_data["tag"],
        )
        .decode("utf-8")
    )


def dumps_encrypted_data(encrypted_data: EncryptedData) -> str:
    return json.dumps(
        {
            key: value.hex() if isinstance(value, bytes) else value
            for key, value in encrypted_data.items()
        },
    )


def loads_encrypted_data(encrypted_data_raw: str) -> EncryptedData:
    data = json.loads(encrypted_data_raw)

    return EncryptedData(
        data=bytes.fromhex(data["data"]),
        tag=bytes.fromhex(data["tag"]),
        nonce=bytes.fromhex(data["nonce"]),
    )


def dump_encrypted_data(encrypted_data: EncryptedData, file: Path) -> None:
    tmp_file_fd, tmp_file = tempfile.mkstemp(prefix="encrypted_data_")

    os.write(
        tmp_file_fd,
        dumps_encrypted_data(encrypted_data).encode("utf-8"),
    )

    os.close(tmp_file_fd)
    os.rename(tmp_file, file)


def load_encrypted_data(file: Path) -> EncryptedData:
    with file.open(mode="r", encoding="utf-8") as file_fd:
        return loads_encrypted_data(file_fd.read())
