"""Small logging utilities for long-running corpus embedding jobs."""

from __future__ import annotations

import threading
import time
from pathlib import Path


def fmt_duration(seconds: float) -> str:
    seconds = int(max(0, seconds))
    hours, rem = divmod(seconds, 3600)
    minutes, secs = divmod(rem, 60)
    return f"{hours:d}:{minutes:02d}:{secs:02d}"


class Logger:
    """Writes a line to stdout and, optionally, to a log file."""

    def __init__(self, log_file: Path | None) -> None:
        self._handle = None
        if log_file is not None:
            log_file.parent.mkdir(parents=True, exist_ok=True)
            self._handle = log_file.open("a", encoding="utf-8")
        self._lock = threading.Lock()

    def log(self, message: str) -> None:
        line = f"[embed-corpus {time.strftime('%Y-%m-%dT%H:%M:%S')}] {message}"
        with self._lock:
            print(line, flush=True)
            if self._handle is not None:
                self._handle.write(line + "\n")
                self._handle.flush()

    def close(self) -> None:
        if self._handle is not None:
            self._handle.close()

