#!/usr/bin/env python3
"""Validate Incident dashboard view wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "status_schema": [
        ("web/dashboard/src/app.js", "incident_view"),
        ("web/dashboard/src/app.js", "buildIncidentView"),
        ("web/dashboard/src/app.js", "dashboard_incident_view.v1"),
        ("web/dashboard/src/reporting_operations.js", "incidentViewRows"),
    ],
    "errors": [
        ("web/dashboard/src/reporting_operations.js", "Incident errors"),
        ("web/dashboard/src/reporting_operations.js", "errors:"),
    ],
    "rate_limits": [
        ("web/dashboard/src/app.js", "principal_quota_requests_rejected"),
        ("web/dashboard/src/app.js", "principal_quota_body_bytes_rejected"),
        ("web/dashboard/src/app.js", "principal_quota_queue_rejected"),
        ("web/dashboard/src/reporting_operations.js", "Rate limits"),
        ("web/dashboard/src/reporting_operations.js", "rate limits:"),
    ],
    "actor_busy": [
        ("web/dashboard/src/app.js", "actor_busy"),
        ("web/dashboard/src/app.js", "cortexdb_actor_queue_depth"),
        ("web/dashboard/src/reporting_operations.js", "Actor busy"),
        ("web/dashboard/src/reporting_operations.js", "actor busy:"),
    ],
    "storage_warnings": [
        ("web/dashboard/src/app.js", "storage_warnings"),
        ("web/dashboard/src/app.js", "storage_validation_failed"),
        ("web/dashboard/src/reporting_operations.js", "Storage warnings"),
        ("web/dashboard/src/reporting_operations.js", "storage warnings:"),
    ],
    "backup_failures": [
        ("web/dashboard/src/app.js", "backup_failures"),
        ("web/dashboard/src/app.js", "backup_blocked_by_validation"),
        ("web/dashboard/src/reporting_operations.js", "Backup failures"),
        ("web/dashboard/src/reporting_operations.js", "backup failures:"),
    ],
    "docs": [
        ("docs/DASHBOARD_UI.md", "Incident View"),
        ("docs/DASHBOARD_UI.md", "dashboard_incident_view.v1"),
        ("docs/DASHBOARD_UI.md", "actor busy status"),
        ("docs/DASHBOARD_UI.md", "storage warnings"),
        ("docs/DASHBOARD_UI.md", "backup failures"),
    ],
    "plan": [
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "### Epic 119. Incident View"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Status: done"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "make incident-view-check"),
    ],
}


def read(path: Path) -> str:
    try:
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
        print(f"incident view check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"incident view check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
