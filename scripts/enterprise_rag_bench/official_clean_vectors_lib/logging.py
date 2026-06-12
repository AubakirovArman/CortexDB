from __future__ import annotations

from progress_logging import ProgressLogger


LOGGER = ProgressLogger("official-clean-vectors")


def log(message: str) -> None:
    LOGGER.log(message)
