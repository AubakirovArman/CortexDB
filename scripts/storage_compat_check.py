#!/usr/bin/env python3
"""Run the storage durability and compatibility evidence matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "docs/STORAGE_COMPATIBILITY.md": (
        "make storage-format-freeze-check",
        "storage_format_freeze_v1.json",
        "current-version backup restored by next-version code",
        "historical backup restore fixture",
        "corruption of `.acs`, `.acb`, `.aci`, `.acv`, and `.ach`",
        "repair dry-run vs repair apply",
        "Strict And Best-Effort Recovery",
    ),
    "docs/BACKUP_RESTORE.md": (
        "make backup-drill-check",
        "target/backup-drill/report.json",
        "current-version backup restored by next-version code",
        "backup archive corruption tests",
        "restore drill trend across releases",
        "Encrypted backup is available as a local passphrase archive MVP",
    ),
    "docs/RPO_RTO.md": (
        "Strict",
        "Balanced",
        "make crash-fault-check",
    ),
    "docs/CRASH_SIMULATION.md": (
        "make crash-fault-check",
        "make chaos-restart-check",
        "corruption of `.acs`, `.acb`, `.aci`, `.acv`, and `.ach`",
    ),
    "docs/UPGRADE_MIGRATION.md": (
        "compatibility_matrix_v1.json",
        "upgrade/downgrade matrix",
        "historical restore fixture",
        "previous-release direct database",
        "migration_upgrade_matrix_v2_check.py",
        "backup-drill",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "storage_format_freeze",
        "command": ["make", "storage-format-freeze-check"],
        "covers": [
            "ACLOG/ACS/ACB/ACI/ACV/ACH/manifest freeze contract",
            "Rust storage constants match freeze fixture",
        ],
    },
    {
        "name": "migration_compatibility",
        "command": ["make", "migration-compatibility-check"],
        "covers": ["storage/API/SDK compatibility fixture", "upgrade/downgrade matrix"],
    },
    {
        "name": "backup_drill",
        "command": ["make", "backup-drill-check"],
        "covers": ["current-version backup restore drill", "validate restored copy"],
    },
    {
        "name": "backup_archive_corruption",
        "command": ["cargo", "test", "-p", "cortex-engine", "--test", "backup_restore", "corrupt_backup"],
        "covers": ["corrupted backup segment rejection", "corrupted backup manifest rejection"],
    },
    {
        "name": "crash_fault",
        "command": ["make", "crash-fault-check"],
        "covers": [
            "interrupted checkpoint/compact aftermath",
            "corruption matrix",
            "repair apply",
        ],
    },
    {
        "name": "chaos_restart",
        "command": ["make", "chaos-restart-check"],
        "covers": ["process kill/restart around writes, flushes, and compacts"],
    },
    {
        "name": "repair_dry_run",
        "command": ["cargo", "test", "-p", "cortex-engine", "--test", "repair_tests", "dry_run"],
        "covers": ["repair dry-run does not mutate files", "repair apply mutates safely"],
    },
    {
        "name": "cli_repair_dry_run",
        "command": ["cargo", "test", "-p", "cortex-cli", "repair_dry_run"],
        "covers": ["CLI dry-run flag", "CLI apply path"],
    },
)


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def git_sha(repo: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return "unknown"
    return result.stdout.strip()


def run_suite(repo: Path, root: Path, suite: dict[str, Any]) -> dict[str, Any]:
    log_path = root / f"{suite['name']}.log"
    started_at = utc_now()
    result = subprocess.run(
        suite["command"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    output = result.stdout
    if result.stderr:
        output += "\n--- stderr ---\n" + result.stderr
    log_path.write_text(output, encoding="utf-8")
    return {
        "name": suite["name"],
        "status": "passed" if result.returncode == 0 else "failed",
        "exit_code": result.returncode,
        "started_at": started_at,
        "finished_at": utc_now(),
        "command": suite["command"],
        "log": str(log_path),
        "covers": suite["covers"],
    }


def check_docs(repo: Path) -> dict[str, Any]:
    missing: list[str] = []
    for relative, terms in DOC_REQUIREMENTS.items():
        path = repo / relative
        try:
            text = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            missing.append(f"{relative}: missing file")
            continue
        for term in terms:
            if term not in text:
                missing.append(f"{relative}: missing {term!r}")
    return {
        "name": "storage_compat_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": [
            "storage compatibility doc",
            "backup/RPO/RTO boundaries",
            "strict/best-effort recovery docs",
            "corruption and repair evidence docs",
        ],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("storage compatibility self-test failed: duplicate suite names")
        return 1
    required = {
        "storage_format_freeze",
        "migration_compatibility",
        "backup_drill",
        "backup_archive_corruption",
        "crash_fault",
        "repair_dry_run",
    }
    missing = sorted(required.difference(names))
    if missing:
        print(f"storage compatibility self-test failed: missing suites {missing}")
        return 1
    if not DOC_REQUIREMENTS:
        print("storage compatibility self-test failed: no doc requirements")
        return 1
    print("storage compatibility self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/storage-compat")
    parser.add_argument("--report", default="target/storage-compat/report.json")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    repo = repo_root()
    root = repo / args.root
    report_path = repo / args.report
    root.mkdir(parents=True, exist_ok=True)

    started_at = utc_now()
    doc_suite = check_docs(repo)
    suites = [doc_suite, *[run_suite(repo, root, suite) for suite in SUITES]]
    status = "passed" if all(suite["status"] == "passed" for suite in suites) else "failed"
    report = {
        "schema_version": 1,
        "status": status,
        "git_sha": git_sha(repo),
        "started_at": started_at,
        "finished_at": utc_now(),
        "suites": suites,
        "artifacts": {
            "report": str(report_path),
            "root": str(root),
            "backup_drill_report": "target/backup-drill/report.json",
            "crash_fault_report": "target/crash-fault/report.json",
            "chaos_restart_report": "target/chaos-restart/report.json",
            "migration_historical_restore_report": "target/migration-historical-restore/report.json",
            "migration_upgrade_matrix_v2_report": "target/migration-upgrade-matrix-v2/report.json",
            "migration_fixture": "fixtures/migration/compatibility_matrix_v1.json",
        },
        "boundary": {
            "proves": [
                "storage compatibility evidence is repeatable locally",
                "historical backup fixtures restore with the current binary",
                "previous-release direct database fixtures open and accept current writes",
                "current checkout can restore and validate current-version backups",
                "corrupted backup archives are rejected during restore",
                "known storage file corruption is detected",
                "interrupted checkpoint/compact aftermath is covered by tests",
                "repair dry-run and apply behavior are both covered",
            ],
            "does_not_prove": [
                "online rolling upgrade",
                "in-place downgrade",
                "remote object-store restore",
                "encrypted backup restore",
                "kill injection at every internal checkpoint byte boundary",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"storage compatibility check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"storage compatibility check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
