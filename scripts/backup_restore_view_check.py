#!/usr/bin/env python3
"""Validate Backup/Restore dashboard view wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path



APP_SOURCE_FILES = (
    Path("web/dashboard/src/app_state.js"),
    Path("web/dashboard/src/app_api.js"),
    Path("web/dashboard/src/app_access.js"),
    Path("web/dashboard/src/app_status.js"),
    Path("web/dashboard/src/app_incidents.js"),
    Path("web/dashboard/src/app_status_summaries.js"),
    Path("web/dashboard/src/app_slo_backup.js"),
    Path("web/dashboard/src/app_bindings.js"),
    Path("web/dashboard/src/app.js"),
)


def read_app_sources() -> str:
    return "\n".join(path.read_text(encoding="utf-8") for path in APP_SOURCE_FILES)

REQUIRED_MARKERS = {
    "status_schema": [
        ("web/dashboard/src/app.js", "backup_restore_view"),
        ("web/dashboard/src/app.js", "buildBackupRestoreView"),
        ("web/dashboard/src/app.js", "dashboard_backup_restore.v1"),
    ],
    "latest_backup": [
        ("web/dashboard/src/app.js", "cortexdb_backup_latest_age_seconds"),
        ("web/dashboard/src/reporting_operations.js", "Latest backup"),
        ("web/dashboard/src/reporting_operations.js", "latest backup:"),
    ],
    "restore_drill": [
        ("web/dashboard/src/app.js", "cortexdb backup-drill"),
        ("web/dashboard/src/app.js", "make backup-restore-production-pack-check"),
        ("web/dashboard/src/reporting_operations.js", "Restore drill"),
        ("web/dashboard/src/reporting_operations.js", "restore drill:"),
    ],
    "offsite": [
        ("web/dashboard/src/app.js", "cortexdb backup-offsite-stage"),
        ("web/dashboard/src/app.js", "make backup-offsite-check"),
        ("web/dashboard/src/reporting_operations.js", "Offsite status"),
        ("web/dashboard/src/reporting_operations.js", "offsite status:"),
    ],
    "rpo_rto": [
        ("web/dashboard/src/app.js", "rpo_budget_seconds"),
        ("web/dashboard/src/app.js", "rto_evidence_gate"),
        ("web/dashboard/src/reporting_operations.js", "RPO/RTO"),
        ("web/dashboard/src/reporting_operations.js", "RPO ${backupRestore.rpo_rto?.rpo_budget_seconds || 86400}s"),
    ],
    "docs": [
        ("docs/archive/DASHBOARD_UI.md", "Backup/Restore View"),
        ("docs/archive/DASHBOARD_UI.md", "latest backup age"),
        ("docs/archive/DASHBOARD_UI.md", "restore drill status"),
        ("docs/archive/DASHBOARD_UI.md", "offsite status"),
        ("docs/archive/DASHBOARD_UI.md", "RPO/RTO posture"),
    ],
    "plan": [
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "### Epic 118. Backup/Restore View"),
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Status: done"),
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "make backup-restore-view-check"),
    ],
}


def read(path: Path) -> str:
    try:
        if path == Path("web/dashboard/src/app.js"):
            return read_app_sources()
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except RuntimeError as error:
        print(f"backup/restore view check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"backup/restore view check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
