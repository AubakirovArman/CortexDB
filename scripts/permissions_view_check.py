#!/usr/bin/env python3
"""Validate Permissions View dashboard wiring."""

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
    "html": [
        ("web/dashboard/src/index.html", "href=\"/dashboard/permissions\""),
        ("web/dashboard/src/index.html", "id=\"permissions-report\""),
    ],
    "state": [
        ("web/dashboard/src/app.js", "dashboard_permissions.v1"),
        ("web/dashboard/src/app.js", "tenant"),
        ("web/dashboard/src/app.js", "role"),
        ("web/dashboard/src/app.js", "token_active"),
        ("web/dashboard/src/app.js", "token_visible: false"),
        ("web/dashboard/src/app.js", "selected_scopes"),
        ("web/dashboard/src/app.js", "server_token_policy"),
        ("web/dashboard/src/app.js", "anonymous_synthetic_view"),
        ("web/dashboard/src/app.js", "denials"),
    ],
    "renderer": [
        ("web/dashboard/src/reporting_operations.js", "renderPermissionsView"),
        ("web/dashboard/src/reporting_operations.js", "Permissions explorer"),
        ("web/dashboard/src/reporting_operations.js", "Token / role / scope / AgentView"),
        ("web/dashboard/src/reporting_operations.js", "Tenant"),
        ("web/dashboard/src/reporting_operations.js", "Role"),
        ("web/dashboard/src/reporting_operations.js", "Token active"),
        ("web/dashboard/src/reporting_operations.js", "Token visible"),
        ("web/dashboard/src/reporting_operations.js", "Scope probes"),
        ("web/dashboard/src/reporting_operations.js", "AgentView policy"),
        ("web/dashboard/src/reporting_operations.js", "Denials"),
    ],
    "docs": [
        ("docs/archive/DASHBOARD_UI.md", "Permissions Explorer"),
        ("docs/archive/DASHBOARD_UI.md", "denials"),
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Epic 115. Permissions View"),
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
        print(f"permissions view check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"permissions view check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
