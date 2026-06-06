#!/usr/bin/env python3
"""Validate single-node SLO dashboard source wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "html_panel": [
        ("web/dashboard/src/index.html", "id=\"slo-report\""),
        ("web/dashboard/src/index.html", "Single-node SLO dashboard"),
        ("web/dashboard/src/index.html", "Availability, latency, backup freshness, validation status, and error budget"),
        ("web/dashboard/src/index.html", "/dashboard/assets/v1/reporting_slo.js"),
    ],
    "status_payload": [
        ("web/dashboard/src/app.js", "dashboard_slo.v1"),
        ("web/dashboard/src/app.js", "buildSloDashboard"),
        ("web/dashboard/src/app.js", "availability"),
        ("web/dashboard/src/app.js", "latency"),
        ("web/dashboard/src/app.js", "backup_freshness"),
        ("web/dashboard/src/app.js", "validation_status"),
        ("web/dashboard/src/app.js", "error_budget"),
        ("web/dashboard/src/app.js", "renderSloDashboard"),
    ],
    "renderer": [
        ("web/dashboard/src/reporting_slo.js", "renderSloDashboard"),
        ("web/dashboard/src/reporting_slo.js", "#slo-report"),
        ("web/dashboard/src/reporting_slo.js", "Availability"),
        ("web/dashboard/src/reporting_slo.js", "Latency"),
        ("web/dashboard/src/reporting_slo.js", "Backup freshness"),
        ("web/dashboard/src/reporting_slo.js", "Validation status"),
        ("web/dashboard/src/reporting_slo.js", "Error budget"),
    ],
    "asset_wiring": [
        ("scripts/dashboard_build.py", "reporting_slo.js"),
        ("crates/cortex-server/src/dashboard.rs", "reporting_slo.js"),
        ("crates/cortex-server/src/dashboard_tests.rs", "reporting_slo.js"),
    ],
    "docs": [
        ("docs/DASHBOARD_UI.md", "Single-node SLO Dashboard"),
        ("docs/SINGLE_NODE_SLO.md", "dashboard_slo.v1"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Epic 110. Single-node SLO Dashboard"),
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
        print(f"single-node SLO dashboard check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"single-node SLO dashboard check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
