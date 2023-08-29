from hashlib import blake2b
from typing import Any

_KEY_LEN = 16

Key = str
KeyHash = bytes


def hash_key(key: Key, /) -> KeyHash:
    # https://docs.python.org/3/library/hashlib.html#keyed-hashing
    return blake2b(key.encode("utf-8"), digest_size=_KEY_LEN).digest()


def check_keyhash(key: Any, /) -> KeyHash:
    if not isinstance(key, bytes):
        raise RuntimeError("Key must be bytes type")

    if len(key) != _KEY_LEN:
        raise RuntimeError("Key length must be 16")

    return key
