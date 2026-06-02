#!/usr/bin/env python3
"""Build the supported backup/restore production-pack evidence report."""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--backup-drill-report", required=True)
    parser.add_argument("--backup-offsite-report", required=True)
    parser.add_argument("--output", required=True)
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
            ["git", "rev-parse", "--short", "HEAD"], text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def validate_backup_drill(report: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    evidence = report.get("evidence", {})
    trend = report.get("restore_drill_trend", [])
    require(report.get("status") == "ok", "backup-drill status is not ok", errors)
    require(
        isinstance(trend, list) and len(trend) >= 3,
        "backup-drill trend needs 3+ drills",
        errors,
    )
    require(
        evidence.get("latest_backup_restored_and_readable") is True,
        "latest backup restore/readback evidence missing",
        errors,
    )
    require(
        "backups_removed=" in str(report.get("prune", "")),
        "backup prune output missing backups_removed",
        errors,
    )
    return {
        "drill_count": len(trend) if isinstance(trend, list) else 0,
        "latest_validate": report.get("latest_validate"),
        "latest_payload": report.get("latest_payload"),
        "prune": report.get("prune"),
        "restore_drill_trend": trend,
    }


def validate_offsite(report: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    require(report.get("status") == "ok", "backup-offsite status is not ok", errors)
    require(
        report.get("staged_copy_validated") is True,
        "offsite staged copy was not validated",
        errors,
    )
    require(
        report.get("preflight_restore_completed") is True,
        "offsite preflight restore missing",
        errors,
    )
    require(
        report.get("payload_readable_after_stage") is True,
        "offsite readback evidence missing",
        errors,
    )
    return {
        "backup_id": report.get("backup_id"),
        "staged_path": report.get("staged_path"),
        "staged_validate_output": report.get("staged_validate_output"),
        "payload_readable_after_stage": report.get("payload_readable_after_stage"),
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    started_ms = int(time.time() * 1000)
    drill_path = Path(args.backup_drill_report)
    offsite_path = Path(args.backup_offsite_report)
    errors: list[str] = []
    drill_summary = validate_backup_drill(load_json(drill_path), errors)
    offsite_summary = validate_offsite(load_json(offsite_path), errors)
    finished_ms = int(time.time() * 1000)
    return {
        "status": "ok" if not errors else "failed",
        "git_sha": git_sha(),
        "generated_unix_ms": finished_ms,
        "generation_duration_ms": finished_ms - started_ms,
        "inputs": {
            "backup_drill_report": str(drill_path),
            "backup_offsite_report": str(offsite_path),
            "encrypted_backup_gate": "make encrypted-backup-check",
        },
        "supported_workflow": [
            "backup",
            "restore",
            "backup-drill",
            "backup-prune",
            "backup-offsite-stage",
            "backup-encrypted",
            "restore-encrypted",
        ],
        "rpo_boundary": {
            "strict_wal": "acknowledged writes are intended durable when local fsync succeeds",
            "balanced_wal": "latest unsynced group can be lost on process or host failure",
            "backup_restore": "data loss is bounded by age of latest validated backup/offsite staged copy",
        },
        "rto_evidence": {
            "local_restore_drill": drill_summary,
            "offsite_preflight_restore": offsite_summary,
            "note": "RTO is measured locally by these gates and depends on database size and storage device.",
        },
        "encrypted_backup_evidence": {
            "gate": "make encrypted-backup-check",
            "covers": [
                "encrypted archive roundtrip",
                "wrong passphrase rejection",
                "corrupt ciphertext rejection",
                "CLI backup-encrypted/restore-encrypted roundtrip",
            ],
        },
        "errors": errors,
    }


def main() -> None:
    args = parse_args()
    report = build_report(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    if report["status"] != "ok":
        raise SystemExit(
            "backup/restore production pack failed: " + "; ".join(report["errors"])
        )
    print(f"backup/restore production pack evidence written to {output}")


if __name__ == "__main__":
    main()
