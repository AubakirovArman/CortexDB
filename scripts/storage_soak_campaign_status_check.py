#!/usr/bin/env python3
"""Regression checks for storage_soak_campaign_status.py."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run_status(root: Path, *, stale_minutes: float, require_active: bool = True) -> subprocess.CompletedProcess[str]:
    args = [
        sys.executable,
        "scripts/storage_soak_campaign_status.py",
        "--pid-file",
        str(root / "pid"),
        "--campaign",
        str(root / "campaign.json"),
        "--history",
        str(root / "history.json"),
        "--soak-root",
        str(root / "soak"),
        "--target-hours",
        "72",
        "--max-stale-minutes",
        str(stale_minutes),
        "--output",
        str(root / "status.json"),
        "--format",
        "json",
    ]
    if require_active:
        args.append("--require-active")
    return subprocess.run(args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)


def write_running_fixture(root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "pid").write_text(f"{os.getpid()}\n", encoding="utf-8")
    soak = root / "soak"
    (soak / "db").mkdir(parents=True)
    (soak / "backups" / "cycle-2.tar").mkdir(parents=True)
    (soak / "restores" / "cycle-2").mkdir(parents=True)


def set_tree_mtime(path: Path, timestamp: float) -> None:
    for child in sorted(path.rglob("*"), reverse=True):
        os.utime(child, (timestamp, timestamp))
    os.utime(path, (timestamp, timestamp))


def load_status(root: Path) -> dict[str, object]:
    return json.loads((root / "status.json").read_text(encoding="utf-8"))


def check_fresh_active_run_passes() -> None:
    with tempfile.TemporaryDirectory(prefix="cortexdb-soak-status-") as tmp:
        root = Path(tmp)
        write_running_fixture(root)
        result = run_status(root, stale_minutes=60)
        assert result.returncode == 0, result.stdout
        status = load_status(root)
        assert status["healthy"] is True
        active = status["active_progress"]
        assert isinstance(active, dict)
        assert active["latest_cycle"] == 2
        assert active["seconds_since_update"] is not None


def check_stale_active_run_fails() -> None:
    with tempfile.TemporaryDirectory(prefix="cortexdb-soak-status-") as tmp:
        root = Path(tmp)
        write_running_fixture(root)
        set_tree_mtime(root / "soak", time.time() - 3600)
        result = run_status(root, stale_minutes=0.001)
        assert result.returncode == 1, result.stdout
        status = load_status(root)
        assert status["healthy"] is False
        active = status["active_progress"]
        assert isinstance(active, dict)
        assert int(active["seconds_since_update"]) > 0


def main() -> int:
    check_fresh_active_run_passes()
    check_stale_active_run_fails()
    print("storage soak campaign status check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
