#!/usr/bin/env python3
"""Validate HTTP contract and operations evidence files."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_FILES = [
    Path("docs/openapi.yaml"),
    Path("docs/API.md"),
    Path("docs/API_ERROR_TAXONOMY.md"),
    Path("docs/AUTH.md"),
    Path("crates/cortex-server/src/lib.rs"),
    Path("crates/cortex-server/src/audit.rs"),
    Path("crates/cortex-server/src/tests/security_tests.rs"),
    Path("crates/cortex-server/src/tests/auth_policy_tests.rs"),
    Path("crates/cortex-server/src/tests/error_taxonomy_tests.rs"),
]

REQUIRED_MARKERS = {
    "request_id_propagation": [
        ("crates/cortex-server/src/lib.rs", "x-request-id"),
        ("crates/cortex-server/src/audit.rs", "request_id"),
        ("crates/cortex-server/src/tests/security_tests.rs", "x_request_id_is_propagated_or_generated"),
    ],
    "auth_roles": [
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "data_token_can_access_data_routes_and_health"),
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "data_token_cannot_access_admin_routes"),
    ],
    "rate_limit": [
        ("crates/cortex-server/src/tests/security_tests.rs", "rate_limit_returns_typed_429_when_enabled"),
        ("docs/API_ERROR_TAXONOMY.md", "rate_limited"),
    ],
    "cors": [
        ("crates/cortex-server/src/tests/security_tests.rs", "cors_preflight_is_only_enabled_for_configured_origin"),
        ("docs/AUTH.md", "CORTEXDB_CORS_ALLOW_ORIGIN"),
    ],
    "audit_redaction": [
        ("crates/cortex-server/src/tests/security_tests.rs", "audit_log_file_records_route_metadata_without_query"),
        ("docs/AUTH.md", "redaction-check"),
    ],
    "typed_errors": [
        ("crates/cortex-server/src/tests/error_taxonomy_tests.rs", "all_router_errors_have_stable_codes_and_statuses"),
        ("docs/openapi.yaml", "ErrorResponse"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    for path in REQUIRED_FILES:
        if not path.exists():
            failures.append(f"missing required file: {path}")

    checks: dict[str, bool] = {}
    for check_name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            path = Path(file_name)
            if marker not in read(path):
                failures.append(f"{check_name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[check_name] = ok

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
        print(f"http contract ops check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"http contract ops check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
