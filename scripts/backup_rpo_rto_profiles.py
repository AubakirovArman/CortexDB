#!/usr/bin/env python3
"""Measure local backup/restore RPO/RTO profile evidence."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any


PROFILES: tuple[tuple[str, int, int], ...] = (
    ("small", 10, 128),
    ("medium", 100, 512),
    ("large", 500, 1024),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/backup-rpo-rto")
    parser.add_argument("--report", default="target/backup-rpo-rto/report.json")
    parser.add_argument("--cli-bin", default="target/debug/cortexdb")
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def git_sha(root: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def run_command(args: list[str], cwd: Path) -> str:
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise SystemExit(
            f"command failed ({result.returncode}): {' '.join(args)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result.stdout.strip()


def timed_command(args: list[str], cwd: Path) -> tuple[str, int]:
    start = time.perf_counter_ns()
    output = run_command(args, cwd)
    duration_ms = (time.perf_counter_ns() - start) // 1_000_000
    return output, duration_ms


def ensure_cli(cli_bin: Path, root: Path) -> Path:
    if not cli_bin.is_absolute():
        cli_bin = root / cli_bin
    if cli_bin.exists():
        return cli_bin
    run_command(["cargo", "build", "-q", "-p", "cortex-cli"], root)
    if not cli_bin.exists():
        raise SystemExit(f"cortexdb binary was not built: {cli_bin}")
    return cli_bin


def payload(profile: str, cell_id: int, payload_bytes: int) -> str:
    header = f"scope=ops\nstatus=ready\nprofile={profile}\ncell={cell_id}\n"
    filler_len = max(0, payload_bytes - len(header))
    return header + ("x" * filler_len)


def parse_kv(output: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for part in output.split():
        if "=" in part:
            key, value = part.split("=", 1)
            values[key] = value
    return values


def put_cells(cli: Path, db: Path, profile: str, cells: int, payload_bytes: int, root: Path) -> None:
    for cell_id in range(1, cells + 1):
        run_command(
            [str(cli), "put", str(db), str(cell_id), payload(profile, cell_id, payload_bytes)],
            root,
        )


def run_profile(cli: Path, root: Path, profile: str, cells: int, payload_bytes: int) -> dict[str, Any]:
    profile_root = root / profile
    db = profile_root / "db"
    backup = profile_root / "backup"
    restore = profile_root / "restore"
    dry_run_target = profile_root / "dry-run-target"
    shutil.rmtree(profile_root, ignore_errors=True)
    profile_root.mkdir(parents=True)

    put_cells(cli, db, profile, cells, payload_bytes, root)
    run_command([str(cli), "flush", str(db)], root)
    wal_tail_id = cells + 1
    post_backup_id = cells + 2
    run_command(
        [str(cli), "put", str(db), str(wal_tail_id), payload(profile, wal_tail_id, payload_bytes)],
        root,
    )

    backup_output, backup_ms = timed_command([str(cli), "backup", str(db), str(backup)], root)
    dry_run_output, dry_run_ms = timed_command(
        [str(cli), "restore", str(backup), str(dry_run_target), "--dry-run"],
        root,
    )
    if dry_run_target.exists():
        raise SystemExit(f"restore dry-run created target path: {dry_run_target}")

    run_command(
        [str(cli), "put", str(db), str(post_backup_id), payload(profile, post_backup_id, payload_bytes)],
        root,
    )
    restore_output, restore_ms = timed_command([str(cli), "restore", str(backup), str(restore)], root)
    validate_output = run_command([str(cli), "validate", str(restore)], root)
    first_payload = run_command([str(cli), "get", str(restore), "1"], root)
    wal_tail_payload = run_command([str(cli), "get", str(restore), str(wal_tail_id)], root)
    post_backup_payload = run_command([str(cli), "get", str(restore), str(post_backup_id)], root)

    backup_kv = parse_kv(backup_output)
    restore_kv = parse_kv(restore_output)
    dry_run_kv = parse_kv(dry_run_output)
    restored_expected = first_payload != "null" and wal_tail_payload != "null"
    post_backup_excluded = post_backup_payload == "null"
    return {
        "profile": profile,
        "cells_before_backup": cells + 1,
        "payload_bytes": payload_bytes,
        "backup_duration_ms": backup_ms,
        "restore_dry_run_duration_ms": dry_run_ms,
        "restore_duration_ms": restore_ms,
        "backup_files": int(backup_kv.get("files_copied", "0")),
        "backup_bytes": int(backup_kv.get("bytes_copied", "0")),
        "restore_files": int(restore_kv.get("files_copied", "0")),
        "restore_bytes": int(restore_kv.get("bytes_copied", "0")),
        "dry_run_files_checked": int(dry_run_kv.get("files_checked", "0")),
        "dry_run_bytes_checked": int(dry_run_kv.get("bytes_checked", "0")),
        "validate_output": validate_output,
        "restored_expected_cells": restored_expected,
        "post_backup_write_excluded": post_backup_excluded,
        "rpo_boundary": "backup restore includes writes durable before backup starts and excludes later writes",
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = repo_root()
    output_root = Path(args.root)
    report_path = Path(args.report)
    shutil.rmtree(output_root, ignore_errors=True)
    output_root.mkdir(parents=True, exist_ok=True)
    cli = ensure_cli(Path(args.cli_bin), root)

    started = time.time()
    profiles = [run_profile(cli, output_root, *profile) for profile in PROFILES]
    errors = [
        f"{item['profile']}: restore/readback failed"
        for item in profiles
        if not item["restored_expected_cells"] or not item["post_backup_write_excluded"]
    ]
    return {
        "status": "ok" if not errors else "failed",
        "git_sha": git_sha(root),
        "generated_unix_ms": int(time.time() * 1000),
        "duration_ms": int((time.time() - started) * 1000),
        "profile_definitions": [
            {"profile": name, "cells": cells, "payload_bytes": payload_bytes}
            for name, cells, payload_bytes in PROFILES
        ],
        "data_loss_boundary": {
            "strict_wal": "acknowledged writes should be durable when fsync succeeds",
            "backup_restore": "restored data is bounded by the latest validated backup; post-backup writes are not expected in the restored copy",
        },
        "profiles": profiles,
        "errors": errors,
        "report_path": str(report_path),
    }


def main() -> None:
    args = parse_args()
    report = build_report(args)
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with report_path.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    if report["status"] != "ok":
        raise SystemExit("backup RPO/RTO profile check failed: " + "; ".join(report["errors"]))
    print(f"backup RPO/RTO profile evidence written to {report_path}")


if __name__ == "__main__":
    main()
