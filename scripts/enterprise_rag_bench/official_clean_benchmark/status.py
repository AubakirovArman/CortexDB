"""Run logging, subprocess execution, and status-file updates."""

from __future__ import annotations

import datetime as dt
import json
import subprocess
import time
from pathlib import Path

from progress_logging import format_duration

from .constants import MAX_LOG_LINE_CHARS, ROOT


RUN_LOG: Path | None = None
RUN_STATUS: Path | None = None
RUN_STARTED_PERF: float | None = None
RUN_STARTED_AT: str | None = None
RUN_SPLIT_NAME: str | None = None
RUN_QUESTIONS_FILE: Path | None = None
CURRENT_STAGE: str | None = None
CURRENT_STEP: int | None = None
CURRENT_TOTAL_STEPS: int | None = None


def log(message: str) -> None:
    stamp = dt.datetime.now(dt.UTC).isoformat(timespec="seconds")
    line = f"[official-clean {stamp}] {message}"
    print(line, flush=True)
    append_run_log(line)


def append_run_log(line: str) -> None:
    if RUN_LOG is not None:
        RUN_LOG.parent.mkdir(parents=True, exist_ok=True)
        if len(line) > MAX_LOG_LINE_CHARS:
            line = line[:MAX_LOG_LINE_CHARS] + " ... [truncated]"
        with RUN_LOG.open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")


def set_run_log(path: Path) -> None:
    global RUN_LOG
    RUN_LOG = path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("", encoding="utf-8")


def set_run_status(path: Path, started_at: str) -> None:
    global RUN_STATUS, RUN_STARTED_PERF, RUN_STARTED_AT
    RUN_STATUS = path
    RUN_STARTED_PERF = time.perf_counter()
    RUN_STARTED_AT = started_at
    path.parent.mkdir(parents=True, exist_ok=True)


def set_stage_context(stage: str, step: int, total_steps: int) -> None:
    global CURRENT_STAGE, CURRENT_STEP, CURRENT_TOTAL_STEPS
    CURRENT_STAGE = stage
    CURRENT_STEP = step
    CURRENT_TOTAL_STEPS = total_steps


def elapsed_seconds() -> float:
    if RUN_STARTED_PERF is None:
        return 0.0
    return max(0.0, time.perf_counter() - RUN_STARTED_PERF)


def read_status_snapshot(path: Path | None) -> dict[str, object] | None:
    if path is None or not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def read_json_snapshot(path: Path | None) -> dict[str, object] | None:
    if path is None or not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def write_current_status(
    *,
    state: str,
    subprocess_label: str | None = None,
    command: list[str] | None = None,
    pid: int | None = None,
    last_output_line: str | None = None,
    error: str | None = None,
    artifacts: dict[str, Path] | None = None,
    child_status: Path | None = None,
) -> None:
    if RUN_STATUS is None:
        return
    payload: dict[str, object] = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_status.v2",
        "stage": CURRENT_STAGE or "unknown",
        "step": CURRENT_STEP or 0,
        "total_steps": CURRENT_TOTAL_STEPS or 0,
        "state": state,
        "started_at": RUN_STARTED_AT,
        "updated_at": dt.datetime.now(dt.UTC).isoformat(timespec="seconds"),
        "elapsed_seconds": round(elapsed_seconds(), 1),
        "elapsed": format_duration(elapsed_seconds()),
        "run_log": str(RUN_LOG) if RUN_LOG else None,
        "split_name": RUN_SPLIT_NAME,
        "questions_file": str(RUN_QUESTIONS_FILE) if RUN_QUESTIONS_FILE else None,
    }
    if subprocess_label:
        payload["subprocess_label"] = subprocess_label
    if command:
        payload["command"] = " ".join(str(item) for item in command)
    if pid:
        payload["pid"] = pid
    if last_output_line:
        payload["last_output_line"] = last_output_line[-500:]
    if error:
        payload["error"] = error
    if artifacts:
        payload["artifacts"] = {key: str(value) for key, value in artifacts.items()}
    child = read_status_snapshot(child_status)
    if child is not None:
        payload["child_status"] = child
    write_json(RUN_STATUS, payload)


def run_cmd(
    cmd: list[str],
    *,
    cwd: Path = ROOT,
    label: str,
    child_status: Path | None = None,
    artifacts: dict[str, Path] | None = None,
) -> None:
    started = time.perf_counter()
    log(f"begin {label}")
    log("+ " + " ".join(str(item) for item in cmd))
    write_current_status(
        state="running",
        subprocess_label=label,
        command=cmd,
        child_status=child_status,
        artifacts=artifacts,
    )
    process = subprocess.Popen(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    assert process.stdout is not None
    for raw_line in process.stdout:
        line = raw_line.rstrip("\n")
        print(line, flush=True)
        append_run_log(line)
        write_current_status(
            state="running",
            subprocess_label=label,
            command=cmd,
            pid=process.pid,
            last_output_line=line,
            child_status=child_status,
            artifacts=artifacts,
        )
    return_code = process.wait()
    if return_code != 0:
        write_current_status(
            state="failed",
            subprocess_label=label,
            command=cmd,
            pid=process.pid,
            error=f"return_code={return_code}",
            child_status=child_status,
            artifacts=artifacts,
        )
        raise subprocess.CalledProcessError(return_code, cmd)
    elapsed = time.perf_counter() - started
    log(f"done {label} elapsed={elapsed:.1f}s")
    write_current_status(
        state="running",
        subprocess_label=f"{label}: done",
        command=cmd,
        child_status=child_status,
        artifacts=artifacts,
    )


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def size_limit(size: int) -> str:
    if size <= 0:
        raise ValueError("--size must be positive")
    return str(size)


def set_run_metadata(split_name: str, questions_file: Path) -> None:
    global RUN_SPLIT_NAME, RUN_QUESTIONS_FILE
    RUN_SPLIT_NAME = split_name
    RUN_QUESTIONS_FILE = questions_file


def write_status(
    path: Path,
    *,
    stage: str,
    step: int,
    total_steps: int,
    state: str,
    started_at: str,
    error: str | None = None,
    artifacts: dict[str, Path] | None = None,
    child_status: Path | None = None,
) -> None:
    payload: dict[str, object] = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_status.v2",
        "stage": stage,
        "step": step,
        "total_steps": total_steps,
        "state": state,
        "started_at": started_at,
        "updated_at": dt.datetime.now(dt.UTC).isoformat(timespec="seconds"),
        "run_log": str(RUN_LOG) if RUN_LOG else None,
        "elapsed_seconds": round(elapsed_seconds(), 1),
        "elapsed": format_duration(elapsed_seconds()),
        "split_name": RUN_SPLIT_NAME,
        "questions_file": str(RUN_QUESTIONS_FILE) if RUN_QUESTIONS_FILE else None,
    }
    if error:
        payload["error"] = error
    if artifacts:
        payload["artifacts"] = {key: str(value) for key, value in artifacts.items()}
    child = read_status_snapshot(child_status)
    if child is not None:
        payload["child_status"] = child
    write_json(path, payload)
