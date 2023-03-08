from asyncio import sleep
from time import monotonic
from typing import Dict, Generic, TypeVar

from sec_store.key import KeyHash

HashId = TypeVar("HashId")


class KeyHashesStore(Generic[HashId]):
    """Not threadsafe"""

    def __init__(self) -> None:
        self._hashes_by_id: Dict[HashId, KeyHash] = {}
        self._last_access_time: Dict[HashId, int] = {}

    def get(self, hash_id: HashId) -> KeyHash:
        key_hash = self._hashes_by_id[hash_id]
        self._last_access_time[hash_id] = monotonic()

        return key_hash

    def set(self, hash_id: HashId, key_hash: KeyHash):
        if hash_id in self._hashes_by_id:
            raise RuntimeError(f"hash for {hash_id=} already exist")

        self._hashes_by_id[hash_id] = key_hash
        self._last_access_time[hash_id] = key_hash

    def remove(self, hash_id: HashId):
        del self._hashes_by_id[hash_id]
        del self._last_access_time[hash_id]

    def remove_unused(self, seconds: float):
        current_time = monotonic()

        unused_hashes_ids = [
            hash_id
            for hash_id, last_access in self._last_access_time.items()
            if current_time - last_access >= seconds
        ]
        for unused_hash_id in unused_hashes_ids:
            self.remove(unused_hash_id)

    def get_oldest_lifetime(self) -> float:
        if not self._last_access_time:
            return 0

        current_time = monotonic()
        return max(
            current_time - last_access_time
            for last_access_time in self._last_access_time.values()
        )


async def watch_and_delete_unused_keyhases(
    keyhashes_store: KeyHashesStore,
    non_use_time: int,
):
    while True:
        keyhashes_store.remove_unused(non_use_time)
        oldest_lifetime = keyhashes_store.get_oldest_lifetime()
        if not oldest_lifetime:
            await sleep(non_use_time)
            continue

        await sleep(max(non_use_time - oldest_lifetime, 0))
