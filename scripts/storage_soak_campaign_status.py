#!/usr/bin/env python3
"""Print concise storage soak campaign progress."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def process_status(pid_file: Path) -> dict[str, Any]:
    if not pid_file.exists():
        return {"running": False, "pid": None, "elapsed": ""}
    pid = pid_file.read_text(encoding="utf-8").strip()
    if not pid:
        return {"running": False, "pid": None, "elapsed": ""}
    result = subprocess.run(
        ["ps", "-p", pid, "-o", "pid=,etime=,stat=,cmd="],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0 or not result.stdout.strip():
        return {"running": False, "pid": pid, "elapsed": ""}
    parts = result.stdout.strip().split(None, 3)
    return {
        "running": True,
        "pid": parts[0],
        "elapsed": parts[1] if len(parts) > 1 else "",
        "stat": parts[2] if len(parts) > 2 else "",
        "cmd": parts[3] if len(parts) > 3 else "",
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid-file", default="target/storage-soak-history/campaign-24h.pid")
    parser.add_argument("--campaign", default="target/storage-soak-history/campaign.json")
    parser.add_argument("--history", default="target/storage-soak-history/report.json")
    parser.add_argument("--format", choices=["text", "json"], default="text")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    campaign = load_json(ROOT / args.campaign)
    history = load_json(ROOT / args.history)
    evidence = history.get("twenty_four_hour_evidence", {})
    status = {
        "process": process_status(ROOT / args.pid_file),
        "campaign_status": campaign.get("status", "unknown"),
        "completed_runs": campaign.get("completed_runs", 0),
        "target_hours": campaign.get("target_hours", 24.0),
        "total_duration_hours": history.get("total_duration_hours", 0.0),
        "run_count": history.get("run_count", 0),
        "total_cycles": history.get("total_cycles", 0),
        "total_cells_written": history.get("total_cells_written", 0),
        "twenty_four_hour_met": evidence.get("met", False),
        "remaining_seconds": evidence.get("remaining_seconds", 24 * 3600),
    }
    if args.format == "json":
        print(json.dumps(status, indent=2, sort_keys=True))
    else:
        print(f"running={status['process']['running']} pid={status['process']['pid']}")
        print(f"campaign_status={status['campaign_status']} completed_runs={status['completed_runs']}")
        print(
            "history="
            f"runs:{status['run_count']} "
            f"cycles:{status['total_cycles']} "
            f"cells:{status['total_cells_written']} "
            f"hours:{status['total_duration_hours']}"
        )
        print(
            "twenty_four_hour_evidence="
            f"met:{status['twenty_four_hour_met']} "
            f"remaining_seconds:{status['remaining_seconds']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
