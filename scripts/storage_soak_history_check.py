#!/usr/bin/env python3
"""Aggregate storage soak reports into repeatable history evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--soak-report", default="target/storage-soak/report.json")
    parser.add_argument("--history-jsonl", default="target/storage-soak-history/history.jsonl")
    parser.add_argument("--output", default="target/storage-soak-history/report.json")
    parser.add_argument("--min-runs", type=int, default=1)
    parser.add_argument("--min-duration-hours", type=float, default=0.0)
    parser.add_argument(
        "--require-24h",
        action="store_true",
        help="Require at least 24 accumulated soak hours.",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise SystemExit(f"{path} must contain a JSON object")
    return value


def git_sha() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def parse_utc(value: str) -> datetime:
    if value.endswith("Z"):
        value = value[:-1] + "+00:00"
    parsed = datetime.fromisoformat(value)
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def duration_seconds(report: dict[str, Any]) -> int:
    started = parse_utc(str(report["started_at"]))
    finished = parse_utc(str(report["finished_at"]))
    return max(0, int((finished - started).total_seconds()))


def require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def validate_soak_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    cycles = report.get("cycles", [])
    kills = report.get("kill_injections", [])
    require(report.get("status") == "passed", "storage soak status is not passed", errors)
    require(isinstance(cycles, list) and bool(cycles), "storage soak cycles are missing", errors)
    require(
        len(cycles) == int(report.get("cycles_requested", -1)),
        "cycle count does not match cycles_requested",
        errors,
    )
    require(isinstance(kills, list) and len(kills) >= 4, "kill injection evidence missing", errors)
    require(bool(report.get("final_validation", {}).get("ok")), "final validation is not ok", errors)
    fixture = report.get("versioned_restore_fixture", {})
    require(bool(fixture.get("validation_ok")), "versioned restore fixture did not validate", errors)
    for cycle in cycles if isinstance(cycles, list) else []:
        prefix = f"cycle {cycle.get('cycle')}"
        require(bool(cycle.get("validation_ok")), f"{prefix} validation failed", errors)
        require(
            bool(cycle.get("backup_restore", {}).get("validation_ok")),
            f"{prefix} backup restore failed",
            errors,
        )
        partial = cycle.get("partial_wal_repair", {})
        require(bool(partial.get("validation_ok")), f"{prefix} partial WAL repair failed", errors)
        require(
            bool(partial.get("partial_tail_truncated")),
            f"{prefix} partial WAL tail was not truncated",
            errors,
        )
    for item in kills if isinstance(kills, list) else []:
        phase = item.get("phase", "unknown")
        recovery_ok = bool(item.get("validation_ok", item.get("final_restore_validation_ok", False)))
        sentinel_ok = bool(item.get("sentinel_readable", True))
        require(recovery_ok, f"kill injection {phase} did not validate", errors)
        require(sentinel_ok, f"kill injection {phase} sentinel was not readable", errors)
    return errors


def entry_id(entry: dict[str, Any]) -> str:
    stable = json.dumps(
        {
            "git_sha": entry["git_sha"],
            "started_at": entry["started_at"],
            "finished_at": entry["finished_at"],
            "cycles_completed": entry["cycles_completed"],
            "cells_written": entry["cells_written"],
        },
        sort_keys=True,
    ).encode("utf-8")
    return hashlib.sha256(stable).hexdigest()[:16]


def build_entry(report: dict[str, Any]) -> dict[str, Any]:
    cycles = report.get("cycles", [])
    kills = report.get("kill_injections", [])
    entry = {
        "schema_version": 1,
        "git_sha": git_sha(),
        "started_at": report["started_at"],
        "finished_at": report["finished_at"],
        "duration_seconds": duration_seconds(report),
        "cycles_completed": len(cycles),
        "cells_written": sum(len(cycle.get("cells_written", [])) for cycle in cycles),
        "kill_injection_phases": [item.get("phase", "unknown") for item in kills],
        "versioned_restore_fixture": report.get("versioned_restore_fixture", {}).get("release_tag"),
    }
    entry["entry_id"] = entry_id(entry)
    return entry


def read_history(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    entries = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise SystemExit(f"{path}:{line_number} must contain a JSON object")
            entries.append(value)
    return entries


def write_history(path: Path, entries: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for entry in entries:
            handle.write(json.dumps(entry, sort_keys=True) + "\n")


def aggregate(entries: list[dict[str, Any]], min_runs: int, min_hours: float) -> dict[str, Any]:
    total_seconds = sum(int(entry.get("duration_seconds", 0)) for entry in entries)
    total_cycles = sum(int(entry.get("cycles_completed", 0)) for entry in entries)
    total_cells = sum(int(entry.get("cells_written", 0)) for entry in entries)
    errors: list[str] = []
    require(len(entries) >= min_runs, f"history has fewer than {min_runs} runs", errors)
    require(
        total_seconds >= int(min_hours * 3600),
        f"history has fewer than {min_hours:.2f} accumulated hours",
        errors,
    )
    return {
        "schema_version": 1,
        "status": "passed" if not errors else "failed",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "run_count": len(entries),
        "total_cycles": total_cycles,
        "total_cells_written": total_cells,
        "total_duration_seconds": total_seconds,
        "total_duration_hours": round(total_seconds / 3600, 6),
        "longest_run_seconds": max((int(e.get("duration_seconds", 0)) for e in entries), default=0),
        "twenty_four_hour_evidence": {
            "met": total_seconds >= 24 * 3600,
            "required_seconds": 24 * 3600,
            "remaining_seconds": max(0, 24 * 3600 - total_seconds),
        },
        "min_required": {
            "runs": min_runs,
            "duration_hours": min_hours,
        },
        "latest_entry": entries[-1] if entries else None,
        "entries": entries,
        "errors": errors,
    }


def main() -> int:
    args = parse_args()
    min_hours = max(args.min_duration_hours, 24.0 if args.require_24h else 0.0)
    soak_report = load_json(Path(args.soak_report))
    validation_errors = validate_soak_report(soak_report)
    entry = build_entry(soak_report)
    history_path = Path(args.history_jsonl)
    entries = read_history(history_path)
    known_ids = {str(item.get("entry_id")) for item in entries}
    if entry["entry_id"] not in known_ids:
        entries.append(entry)
    entries.sort(key=lambda item: (str(item.get("finished_at", "")), str(item.get("entry_id", ""))))
    write_history(history_path, entries)
    report = aggregate(entries, args.min_runs, min_hours)
    report["status"] = "failed" if validation_errors or report["errors"] else "passed"
    report["current_report_errors"] = validation_errors
    report["inputs"] = {
        "soak_report": args.soak_report,
        "history_jsonl": args.history_jsonl,
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"storage soak history check failed: {output}")
    print(f"storage soak history check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
