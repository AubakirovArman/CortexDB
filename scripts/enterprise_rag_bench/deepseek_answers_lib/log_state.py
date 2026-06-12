from __future__ import annotations

from pathlib import Path

from progress_logging import ProgressLogger

LOGGER = ProgressLogger("answer-runner")


def configure(log_file: Path | None = None, status_file: Path | None = None) -> None:
    global LOGGER
    LOGGER = ProgressLogger("answer-runner", log_file=log_file, status_file=status_file)


def log(message: str) -> None:
    LOGGER.log(message)
