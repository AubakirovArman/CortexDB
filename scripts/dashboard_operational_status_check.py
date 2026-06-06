#!/usr/bin/env python3
"""Validate dashboard operational status view wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "html": [
        ("web/dashboard/src/index.html", "id=\"status-report\""),
        ("web/dashboard/src/index.html", "Operational status"),
        ("web/dashboard/src/index.html", "Health, stats, validation, backup posture, and request error state"),
    ],
    "payload": [
        ("web/dashboard/src/app.js", "dashboard_status.v1"),
        ("web/dashboard/src/app.js", "summarizeStatsResult"),
        ("web/dashboard/src/app.js", "summarizeValidationResult"),
        ("web/dashboard/src/app.js", "summarizeMetricsResult"),
        ("web/dashboard/src/app.js", "backup_latest_age_seconds"),
        ("web/dashboard/src/app.js", "actor_queue_depth"),
        ("web/dashboard/src/app.js", "actor_queue_capacity"),
        ("web/dashboard/src/app.js", "last_request_error"),
    ],
    "renderer": [
        ("web/dashboard/src/reporting_operations.js", "renderOperationalStatus"),
        ("web/dashboard/src/reporting_operations.js", "Health"),
        ("web/dashboard/src/reporting_operations.js", "Current seq"),
        ("web/dashboard/src/reporting_operations.js", "Actor queue"),
        ("web/dashboard/src/reporting_operations.js", "Latest backup"),
        ("web/dashboard/src/reporting_operations.js", "Validation"),
        ("web/dashboard/src/reporting_operations.js", "Last error"),
    ],
    "docs": [
        ("docs/DASHBOARD_UI.md", "Operational Status View"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Epic 111. Dashboard Operational Status View"),
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
        print(f"dashboard operational status check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"dashboard operational status check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
