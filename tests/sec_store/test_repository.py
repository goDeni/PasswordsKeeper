import os
import tempfile
from pathlib import Path

import pytest

from sec_store.exceptions import WrongKeyHashError
from sec_store.key import KeyHash
from sec_store.record import Record
from sec_store.records_repository import RecordsRepository


@pytest.fixture(scope="function")
def repository_file():
    fd, name = tempfile.mkstemp(prefix="tests_")
    try:
        os.close(fd)
        yield Path(name)
    finally:
        os.unlink(name)


@pytest.fixture(scope="function")
def records_count():
    return 0


@pytest.fixture(scope="function")
def records_repository(
    repository_file: Path, keyhash: KeyHash, records_count: int
) -> RecordsRepository:
    rep = RecordsRepository.new(
        file=repository_file,
        keyhash=keyhash,
    )
    for num in range(records_count):
        rep.add_record(
            Record(name=f"name-{num}", description=f"description-{num}", value=str(num))
        )

    return rep


def test_repository_saving(
    records_repository: RecordsRepository,
    repository_file: Path,
    keyhash: KeyHash,
):
    record = Record(
        description="1",
        name="2",
        value="3",
    )
    records_repository.add_record(record)
    assert records_repository.records == [record]
    records_repository.save()

    same_rep = RecordsRepository.open(repository_file, keyhash)
    same_rep.records == records_repository.records


def test_record_edit(
    records_repository: RecordsRepository,
    repository_file: Path,
    keyhash: KeyHash,
):
    new_record_name = "New name"
    records_repository.add_record(
        Record(
            description="1",
            name=new_record_name,
            value="3",
        )
    )

    records_repository.records[0].name = new_record_name
    records_repository.save()

    same_rep = RecordsRepository.open(
        file=repository_file,
        keyhash=keyhash,
    )
    assert same_rep.records[0].name == new_record_name


@pytest.mark.parametrize(
    "records_count",
    [
        0,
        1,
        10,
    ],
)
def test_repository_opening_with_password(
    records_repository: RecordsRepository,
    repository_file: Path,
    keyhash: KeyHash,
):
    records_repository.save()
    opened_rep = RecordsRepository.open(repository_file, keyhash)

    assert opened_rep.records == records_repository.records
    assert opened_rep._id == records_repository._id


@pytest.mark.parametrize(
    "records_count",
    [
        10,
        1,
        0,
    ],
)
def test_repository_opening_with_wrong_password(
    records_repository: RecordsRepository,
    repository_file: Path,
    wrong_keyhash: KeyHash,
):
    records_repository.save()
    with pytest.raises(WrongKeyHashError):
        RecordsRepository.open(repository_file, wrong_keyhash)
