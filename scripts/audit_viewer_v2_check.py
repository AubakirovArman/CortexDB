#!/usr/bin/env python3
"""Validate Audit Viewer v2 dashboard wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "html_filters": [
        ("web/dashboard/src/index.html", "id=\"audit-filter-form\""),
        ("web/dashboard/src/index.html", "id=\"audit-filter-category\""),
        ("web/dashboard/src/index.html", "id=\"audit-filter-severity\""),
        ("web/dashboard/src/index.html", "All categories"),
        ("web/dashboard/src/index.html", "All severities"),
    ],
    "viewer_schema": [
        ("web/dashboard/src/reporting_audit.js", "dashboard_audit_viewer.v2"),
        ("web/dashboard/src/reporting_audit.js", "filters"),
        ("web/dashboard/src/reporting_audit.js", "summary"),
        ("web/dashboard/src/reporting_audit.js", "filtered_events"),
    ],
    "hash_chain": [
        ("web/dashboard/src/reporting_audit.js", "hash_chain_verification"),
        ("web/dashboard/src/reporting_audit.js", "cortexdb audit verify --file $CORTEXDB_AUDIT_LOG_FILE"),
        ("web/dashboard/src/reporting_audit.js", "cli_required"),
    ],
    "redaction_status": [
        ("web/dashboard/src/reporting_audit.js", "redaction_status"),
        ("web/dashboard/src/reporting_audit.js", "browser_redacted"),
        ("web/dashboard/src/reporting_audit.js", "query_visible: false"),
        ("web/dashboard/src/reporting_audit.js", "body_visible: false"),
        ("web/dashboard/src/reporting_audit.js", "token_visible: false"),
    ],
    "docs": [
        ("docs/archive/DASHBOARD_UI.md", "Audit Viewer v2"),
        ("docs/archive/DASHBOARD_UI.md", "filters for safe audit event category and severity"),
        ("docs/archive/DASHBOARD_UI.md", "hash-chain verification guidance"),
        ("docs/archive/DASHBOARD_UI.md", "redaction status"),
    ],
    "plan": [
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "### Epic 116. Audit Viewer v2"),
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Status: done"),
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "make audit-viewer-v2-check"),
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
        print(f"audit viewer v2 check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"audit viewer v2 check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
