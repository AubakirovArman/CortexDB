#!/usr/bin/env python3
"""Start a storage soak campaign as a detached local process."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def running_pid(pid_file: Path) -> int | None:
    if not pid_file.exists():
        return None
    raw = pid_file.read_text(encoding="utf-8").strip()
    if not raw:
        return None
    try:
        pid = int(raw)
    except ValueError:
        return None
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return None
    except PermissionError:
        return pid
    return pid


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid-file", required=True)
    parser.add_argument("--log-file", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if args.command and args.command[0] == "--":
        args.command = args.command[1:]
    if not args.command:
        parser.error("command is required after --")
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    pid_file = ROOT / args.pid_file
    log_file = ROOT / args.log_file
    existing = running_pid(pid_file)
    if existing is not None:
        print(f"storage soak campaign already running: pid={existing}")
        return 0

    pid_file.parent.mkdir(parents=True, exist_ok=True)
    log_file.parent.mkdir(parents=True, exist_ok=True)
    log = log_file.open("ab")
    try:
        process = subprocess.Popen(
            args.command,
            cwd=ROOT,
            stdout=log,
            stderr=subprocess.STDOUT,
            stdin=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception:
        log.close()
        raise
    pid_file.write_text(f"{process.pid}\n", encoding="utf-8")
    print(f"storage soak campaign started: pid={process.pid} log={log_file}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
