#!/usr/bin/env python3
"""Run repeated storage soaks until accumulated history reaches a target."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def run(command: list[str]) -> int:
    return subprocess.run(command, cwd=ROOT, check=False).returncode


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def history_hours(path: Path) -> float:
    report = load_json(path)
    return float(report.get("total_duration_hours", 0.0))


def write_campaign_report(
    path: Path,
    *,
    status: str,
    started_at: str,
    target_hours: float,
    completed_runs: int,
    history_report: Path,
    last_exit_code: int,
) -> None:
    report = load_json(history_report)
    payload = {
        "schema_version": 1,
        "status": status,
        "started_at": started_at,
        "updated_at": utc_now(),
        "target_hours": target_hours,
        "completed_runs": completed_runs,
        "last_exit_code": last_exit_code,
        "history_report": str(history_report),
        "history_total_duration_hours": float(report.get("total_duration_hours", 0.0)),
        "twenty_four_hour_evidence": report.get("twenty_four_hour_evidence", {}),
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_once(args: argparse.Namespace) -> int:
    soak_exit = run(
        [
            sys.executable,
            "scripts/storage_soak_check.py",
            "--root",
            args.soak_root,
            "--report",
            args.soak_report,
            "--cycles",
            str(args.cycles),
            "--cells-per-cycle",
            str(args.cells_per_cycle),
            "--kill-delay-ms",
            str(args.kill_delay_ms),
        ]
    )
    if soak_exit != 0:
        return soak_exit
    return run(
        [
            sys.executable,
            "scripts/storage_soak_history_check.py",
            "--soak-report",
            args.soak_report,
            "--history-jsonl",
            args.history_jsonl,
            "--output",
            args.history_report,
            "--min-runs",
            "1",
            "--min-duration-hours",
            "0",
        ]
    )


def final_gate(args: argparse.Namespace) -> int:
    return run(
        [
            sys.executable,
            "scripts/storage_soak_history_check.py",
            "--soak-report",
            args.soak_report,
            "--history-jsonl",
            args.history_jsonl,
            "--output",
            args.history_report,
            "--min-runs",
            str(args.min_runs),
            "--min-duration-hours",
            str(args.target_hours),
        ]
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target-hours", type=float, default=24.0)
    parser.add_argument("--max-runs", type=int, default=100000)
    parser.add_argument("--min-runs", type=int, default=1)
    parser.add_argument("--cycles", type=int, default=20)
    parser.add_argument("--cells-per-cycle", type=int, default=50)
    parser.add_argument("--kill-delay-ms", type=int, default=15)
    parser.add_argument("--soak-root", default="target/storage-soak")
    parser.add_argument("--soak-report", default="target/storage-soak/report.json")
    parser.add_argument("--history-jsonl", default="target/storage-soak-history/history.jsonl")
    parser.add_argument("--history-report", default="target/storage-soak-history/report.json")
    parser.add_argument("--campaign-report", default="target/storage-soak-history/campaign.json")
    parser.add_argument("--no-run-if-complete", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    started_at = utc_now()
    history_report = ROOT / args.history_report
    campaign_report = ROOT / args.campaign_report
    completed = 0
    last_exit_code = 0

    if args.no_run_if_complete and history_hours(history_report) >= args.target_hours:
        write_campaign_report(
            campaign_report,
            status="passed",
            started_at=started_at,
            target_hours=args.target_hours,
            completed_runs=0,
            history_report=history_report,
            last_exit_code=0,
        )
        return 0

    while completed < args.max_runs:
        if completed > 0 and history_hours(history_report) >= args.target_hours:
            break
        last_exit_code = run_once(args)
        completed += 1
        write_campaign_report(
            campaign_report,
            status="running" if last_exit_code == 0 else "failed",
            started_at=started_at,
            target_hours=args.target_hours,
            completed_runs=completed,
            history_report=history_report,
            last_exit_code=last_exit_code,
        )
        if last_exit_code != 0:
            return last_exit_code
        if history_hours(history_report) >= args.target_hours:
            break

    last_exit_code = final_gate(args)
    status = "passed" if last_exit_code == 0 else "failed"
    write_campaign_report(
        campaign_report,
        status=status,
        started_at=started_at,
        target_hours=args.target_hours,
        completed_runs=completed,
        history_report=history_report,
        last_exit_code=last_exit_code,
    )
    return last_exit_code


if __name__ == "__main__":
    raise SystemExit(main())
