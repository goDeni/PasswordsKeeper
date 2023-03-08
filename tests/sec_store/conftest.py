import tempfile
from pathlib import Path

import pytest

from sec_store.key import KeyHash, hash_key


@pytest.fixture(scope="function")
def keyhash() -> KeyHash:
    return hash_key("Some key for tests")


@pytest.fixture(scope="function")
def wrong_keyhash(keyhash: KeyHash) -> KeyHash:
    _wrong_keyhash = hash_key("Some wrong key for tests")
    assert _wrong_keyhash != keyhash

    return _wrong_keyhash


@pytest.fixture(scope="function")
def tmp_dir():
    with tempfile.TemporaryDirectory(
        prefix="tests_",
    ) as _tmp_dir:
        yield Path(_tmp_dir)


@pytest.fixture(scope="function")
def records_dir():
    with tempfile.TemporaryDirectory(
        prefix="tests_records_",
    ) as _tmp_dir:
        yield Path(_tmp_dir)
