#!/usr/bin/env python3
"""Validate Dashboard role-based UI wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "role_ui_schema": [
        ("web/dashboard/src/index.html", "id=\"role-ui-report\""),
        ("web/dashboard/src/app.js", "roleUiState"),
        ("web/dashboard/src/app.js", "dashboard_role_ui.v1"),
        ("web/dashboard/src/reporting_operations.js", "Role-based UI"),
        ("web/dashboard/src/reporting_operations.js", "roleUiRows"),
    ],
    "admin_ui": [
        ("web/dashboard/src/app.js", "admin_ui"),
        ("web/dashboard/src/reporting_operations.js", "admin UI:"),
        ("web/dashboard/src/index.html", "data-access=\"admin\""),
        ("web/dashboard/src/index.html", "data-action=\"flush\""),
        ("web/dashboard/src/index.html", "data-action=\"compact\""),
    ],
    "data_user_ui": [
        ("web/dashboard/src/app.js", "data_user_ui"),
        ("web/dashboard/src/reporting_operations.js", "data user UI:"),
        ("web/dashboard/src/index.html", "data-access=\"data\""),
        ("web/dashboard/src/index.html", "href=\"/dashboard/search\""),
        ("web/dashboard/src/index.html", "href=\"/dashboard/context\""),
    ],
    "read_only_ui": [
        ("web/dashboard/src/app.js", "read_only_ui"),
        ("web/dashboard/src/reporting_operations.js", "read-only UI:"),
        ("web/dashboard/src/index.html", "id=\"read-only-mode\""),
        ("web/dashboard/src/app.js", "client_side_before_request"),
    ],
    "dangerous_operations": [
        ("web/dashboard/src/index.html", "data-dangerous=\"true\""),
        ("web/dashboard/src/app.js", "refreshDangerousOperationVisibility"),
        ("web/dashboard/src/app.js", "dangerous_operations"),
        ("web/dashboard/src/app.js", "hidden_or_disabled"),
        ("web/dashboard/src/reporting_operations.js", "Dangerous visible"),
        ("web/dashboard/src/reporting_operations.js", "dangerous hidden/disabled:"),
    ],
    "docs": [
        ("docs/DASHBOARD_UI.md", "Role-based Dashboard UI"),
        ("docs/DASHBOARD_UI.md", "dashboard_role_ui.v1"),
        ("docs/DASHBOARD_UI.md", "Hide dangerous operations by role"),
    ],
    "plan": [
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "### Epic 120. Dashboard Role-based UI"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Status: done"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "make dashboard-role-ui-check"),
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
        print(f"dashboard role UI check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"dashboard role UI check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
