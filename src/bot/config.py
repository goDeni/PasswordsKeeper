import os
from pathlib import Path

from dotenv import load_dotenv

load_dotenv("./.env")


def get_bot_token():
    if token := os.environ.get("API_TOKEN"):
        return token

    raise RuntimeError("API_TOKEN variable must be defined")


_ROOT_DIR = Path(".passwords_keeper_bot_data")
_ROOT_DIR.mkdir(exist_ok=True, parents=True)

REPOSITORIES_DIR = _ROOT_DIR.joinpath("repositories")
REPOSITORIES_DIR.mkdir(exist_ok=True)
