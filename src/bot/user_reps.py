from pathlib import Path

from bot.config import REPOSITORIES_DIR
from sec_store.key import KeyHash
from sec_store.records_repository import RecordsRepository


def _get_repository_path(user_id: int) -> Path:
    return Path(REPOSITORIES_DIR, f"rep_{user_id}")


def user_has_repository(user_id: int) -> bool:
    return _get_repository_path(user_id).exists()


def get_user_repository(user_id: int, keyhash: KeyHash) -> RecordsRepository:
    return RecordsRepository.open(
        file=_get_repository_path(user_id),
        keyhash=keyhash,
    )


def initialize_user_repository(user_id: int, keyhash: KeyHash):
    RecordsRepository.new(_get_repository_path(user_id), keyhash).save()
