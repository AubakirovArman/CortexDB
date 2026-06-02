#!/usr/bin/env python3
"""Print concise storage soak campaign progress."""

from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
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


def parse_utc_timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str) or not value:
        return None
    normalized = value.removesuffix("Z") + "+00:00" if value.endswith("Z") else value
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid-file", default="target/storage-soak-history/campaign-24h.pid")
    parser.add_argument("--campaign", default="target/storage-soak-history/campaign.json")
    parser.add_argument("--history", default="target/storage-soak-history/report.json")
    parser.add_argument("--format", choices=["text", "json"], default="text")
    parser.add_argument("--require-active", action="store_true")
    parser.add_argument("--max-stale-minutes", type=float, default=30.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    campaign = load_json(ROOT / args.campaign)
    history = load_json(ROOT / args.history)
    evidence = history.get("twenty_four_hour_evidence", {})
    target_hours = float(campaign.get("target_hours", 24.0))
    total_duration_hours = float(history.get("total_duration_hours", 0.0))
    progress_percent = 0.0
    if target_hours > 0:
        progress_percent = min(100.0, (total_duration_hours / target_hours) * 100.0)
    updated_at = parse_utc_timestamp(campaign.get("updated_at"))
    seconds_since_update = None
    if updated_at is not None:
        seconds_since_update = max(0, int((datetime.now(timezone.utc) - updated_at).total_seconds()))
    process = process_status(ROOT / args.pid_file)
    twenty_four_hour_met = evidence.get("met", False)
    stale = seconds_since_update is None or seconds_since_update > args.max_stale_minutes * 60
    healthy = bool(twenty_four_hour_met or (
        process.get("running")
        and campaign.get("status") == "running"
        and not stale
    ))
    status = {
        "process": process,
        "campaign_status": campaign.get("status", "unknown"),
        "completed_runs": campaign.get("completed_runs", 0),
        "target_hours": target_hours,
        "total_duration_hours": total_duration_hours,
        "progress_percent": round(progress_percent, 4),
        "updated_at": campaign.get("updated_at"),
        "seconds_since_update": seconds_since_update,
        "healthy": healthy,
        "run_count": history.get("run_count", 0),
        "total_cycles": history.get("total_cycles", 0),
        "total_cells_written": history.get("total_cells_written", 0),
        "twenty_four_hour_met": twenty_four_hour_met,
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
            f"hours:{status['total_duration_hours']} "
            f"progress_percent:{status['progress_percent']}"
        )
        print(
            "twenty_four_hour_evidence="
            f"met:{status['twenty_four_hour_met']} "
            f"remaining_seconds:{status['remaining_seconds']}"
        )
        print(
            "watchdog="
            f"healthy:{status['healthy']} "
            f"seconds_since_update:{status['seconds_since_update']}"
        )
    if args.require_active and not healthy:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
