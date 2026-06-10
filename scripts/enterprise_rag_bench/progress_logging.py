"""Shared progress logging for long-running EnterpriseRAG scripts."""

from __future__ import annotations

import datetime as dt
import json
import os
import threading
import time
from pathlib import Path
from typing import Any


MAX_LOG_LINE_CHARS = 4_000


class ProgressLogger:
    def __init__(
        self,
        prefix: str,
        *,
        log_file: Path | None = None,
        status_file: Path | None = None,
    ) -> None:
        self.prefix = prefix
        self.log_file = log_file
        self.status_file = status_file
        self.started_at = now_utc()
        self.started_perf = time.perf_counter()
        self._lock = threading.RLock()
        if self.log_file is not None:
            self.log_file.parent.mkdir(parents=True, exist_ok=True)

    def log(self, message: str) -> None:
        line = f"[{self.prefix} {now_utc()}] {message}"
        print(line, flush=True)
        self.append(line)

    def append(self, line: str) -> None:
        if self.log_file is None:
            return
        if len(line) > MAX_LOG_LINE_CHARS:
            line = line[:MAX_LOG_LINE_CHARS] + " ... [truncated]"
        with self._lock:
            with self.log_file.open("a", encoding="utf-8") as handle:
                handle.write(line + "\n")

    def status(
        self,
        *,
        stage: str,
        state: str,
        step: int | None = None,
        total_steps: int | None = None,
        error: str | None = None,
        **extra: Any,
    ) -> None:
        if self.status_file is None:
            return
        elapsed = max(0.0, time.perf_counter() - self.started_perf)
        payload: dict[str, Any] = {
            "schema_version": "cortexdb.enterprise_rag_bench.progress_status.v1",
            "prefix": self.prefix,
            "stage": stage,
            "state": state,
            "started_at": self.started_at,
            "updated_at": now_utc(),
            "elapsed": format_duration(elapsed),
            "elapsed_seconds": round(elapsed, 1),
            "pid": os.getpid(),
            "log_file": str(self.log_file) if self.log_file else None,
        }
        if step is not None:
            payload["step"] = step
        if total_steps is not None:
            payload["total_steps"] = total_steps
        if error:
            payload["error"] = error
        payload.update(extra)
        with self._lock:
            self.status_file.parent.mkdir(parents=True, exist_ok=True)
            self.status_file.write_text(
                json.dumps(payload, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )

    def step(
        self,
        *,
        stage: str,
        state: str,
        step: int,
        total_steps: int,
        message: str,
        **extra: Any,
    ) -> None:
        self.log(f"{stage}: step {step}/{total_steps} {message}")
        self.status(
            stage=stage,
            state=state,
            step=step,
            total_steps=total_steps,
            **extra,
        )

    def progress(
        self,
        *,
        stage: str,
        completed: int,
        total: int,
        unit: str = "items",
        state: str = "running",
        **extra: Any,
    ) -> None:
        elapsed = max(0.0, time.perf_counter() - self.started_perf)
        pct = (completed / total * 100.0) if total > 0 else 100.0
        rate = (completed / elapsed) if elapsed > 0.0 else 0.0
        remaining = max(0, total - completed)
        eta_seconds = (remaining / rate) if rate > 0.0 else None
        details = " ".join(
            f"{key}={value}"
            for key, value in extra.items()
            if value is not None and key not in {"error"}
        )
        line = (
            f"{stage}: {completed}/{total} {unit} ({pct:.1f}%) "
            f"elapsed={format_duration(elapsed)} rate={rate:.2f}/s "
            f"eta={format_duration(eta_seconds) if eta_seconds is not None else 'unknown'}"
        )
        if details:
            line = f"{line} {details}"
        self.log(line)
        self.status(
            stage=stage,
            state=state,
            completed=completed,
            total=total,
            progress_pct=round(pct, 2),
            elapsed_seconds=round(elapsed, 1),
            rate_per_second=round(rate, 4),
            eta_seconds=round(eta_seconds, 1) if eta_seconds is not None else None,
            **extra,
        )


def now_utc() -> str:
    return dt.datetime.now(dt.UTC).isoformat(timespec="seconds")


def format_duration(seconds: float | None) -> str:
    if seconds is None:
        return "unknown"
    seconds = max(0, int(seconds))
    hours, remainder = divmod(seconds, 3600)
    minutes, secs = divmod(remainder, 60)
    if hours:
        return f"{hours}h{minutes:02d}m{secs:02d}s"
    if minutes:
        return f"{minutes}m{secs:02d}s"
    return f"{secs}s"
