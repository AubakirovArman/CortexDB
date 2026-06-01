#!/usr/bin/env python3
"""Validate Dashboard Product UI evidence wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "read_only_mode": [
        ("web/dashboard/src/index.html", "id=\"read-only-mode\""),
        ("web/dashboard/src/app.js", "guardWriteAllowed"),
        ("web/dashboard/src/app.js", "cortexdb-dashboard-read-only"),
    ],
    "operational_status": [
        ("web/dashboard/src/index.html", "id=\"status-report\""),
        ("web/dashboard/src/app.js", "dashboard_status.v1"),
        ("web/dashboard/src/reporting_operations.js", "renderOperationalStatus"),
    ],
    "audit_readiness": [
        ("web/dashboard/src/index.html", "id=\"audit-report\""),
        ("web/dashboard/src/index.html", "data-action=\"audit-readiness\""),
        ("web/dashboard/src/reporting_audit.js", "dashboard_audit_readiness.v1"),
        ("web/dashboard/src/reporting_audit.js", "renderAuditReadiness"),
        ("docs/DASHBOARD_UI.md", "Audit readiness"),
    ],
    "permissions_view": [
        ("web/dashboard/src/index.html", "href=\"/dashboard/permissions\""),
        ("web/dashboard/src/dashboard_manifest.json", "\"permissions\""),
        ("web/dashboard/src/reporting_operations.js", "renderPermissionsView"),
    ],
    "release_artifacts": [
        ("e2e/dashboard_screenshots.mjs", "permissions"),
        ("docs/DASHBOARD_UI.md", "dashboard-screenshots"),
        ("docs/DASHBOARD_PRODUCT_UI_EVIDENCE.md", "make dashboard-product-check"),
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
        "failures": failures,
        "checks": checks,
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
        print(f"dashboard product check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"dashboard product check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
