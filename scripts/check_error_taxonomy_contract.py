#!/usr/bin/env python3
"""Verify the stable HTTP error taxonomy across docs, OpenAPI, server, and SDK."""

from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml


CODE_STATUS = {
    "bad_request": 400,
    "invalid_tenant": 400,
    "invalid_aql": 400,
    "unknown_field": 400,
    "unsupported_operator": 400,
    "unauthorized": 401,
    "forbidden": 403,
    "permission_denied": 403,
    "not_found": 404,
    "payload_too_large": 413,
    "rate_limited": 429,
    "storage_corruption": 500,
    "internal": 500,
    "database_busy": 503,
    "service_unavailable": 503,
}


REPO = Path(__file__).resolve().parent.parent


def read(relative: str) -> str:
    return (REPO / relative).read_text(encoding="utf-8")


def pascal_case(code: str) -> str:
    return "".join(part.capitalize() for part in code.split("_"))


def openapi_codes() -> list[str]:
    spec = yaml.safe_load(read("docs/openapi.yaml"))
    return (
        spec["components"]["schemas"]["ErrorResponse"]["properties"]["code"]["enum"]
    )


def taxonomy_rows() -> dict[str, int]:
    rows: dict[str, int] = {}
    for status, code in re.findall(
        r"\|\s*`(\d{3})`\s*\|\s*`([a-z_]+)`\s*\|",
        read("docs/archive/API_ERROR_TAXONOMY.md"),
    ):
        rows[code] = int(status)
    return rows


def server_codes() -> set[str]:
    return set(
        re.findall(r'=>\s*"([a-z_]+)"', read("crates/cortex-server/src/responses/errors.rs"))
    )


def sdk_test_codes() -> set[str]:
    return set(re.findall(r'\("([a-z_]+)",\s*ErrorCode::', read("crates/cortex-sdk/src/tests.rs")))


def require_contains_all(relative: str, codes: set[str], failures: list[str]) -> None:
    text = read(relative)
    for code in sorted(codes):
        if code not in text:
            failures.append(f"{relative}: missing error code {code!r}")


def validate() -> list[str]:
    expected = set(CODE_STATUS)
    failures: list[str] = []

    taxonomy = taxonomy_rows()
    if set(taxonomy) != expected:
        failures.append(
            "docs/archive/API_ERROR_TAXONOMY.md codes differ: "
            f"missing={sorted(expected - set(taxonomy))} extra={sorted(set(taxonomy) - expected)}"
        )
    for code, status in CODE_STATUS.items():
        if taxonomy.get(code) != status:
            failures.append(
                f"docs/archive/API_ERROR_TAXONOMY.md status mismatch for {code}: "
                f"expected {status}, got {taxonomy.get(code)}"
            )

    openapi = set(openapi_codes())
    if openapi != expected:
        failures.append(
            "docs/openapi.yaml ErrorResponse enum differs: "
            f"missing={sorted(expected - openapi)} extra={sorted(openapi - expected)}"
        )

    server = server_codes()
    if server != expected:
        failures.append(
            "server ErrorCode::as_str differs: "
            f"missing={sorted(expected - server)} extra={sorted(server - expected)}"
        )

    sdk_codes = sdk_test_codes()
    if sdk_codes != expected:
        failures.append(
            "Rust SDK decoder test differs: "
            f"missing={sorted(expected - sdk_codes)} extra={sorted(sdk_codes - expected)}"
        )

    sdk_types = read("crates/cortex-sdk/src/types/error.rs")
    for code in sorted(expected):
        variant = pascal_case(code)
        if variant not in sdk_types:
            failures.append(f"crates/cortex-sdk/src/types/error.rs: missing ErrorCode::{variant}")

    require_contains_all("docs/API_JSON_SCHEMAS.md", expected, failures)
    require_contains_all("docs/API.md", expected, failures)
    require_contains_all("docs/archive/API_ERROR_TAXONOMY.md", expected, failures)
    require_contains_all("crates/cortex-server/src/tests/error_taxonomy_tests.rs", expected, failures)
    require_contains_all(
        "crates/cortex-server/src/tests/snapshots/"
        "cortex_server__tests__error_response_snapshot_tests__snapshot_all_sdk_visible_error_responses.snap",
        expected,
        failures,
    )

    return failures


def main() -> int:
    failures = validate()
    if failures:
        print("error taxonomy contract check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print("OK: error taxonomy is aligned across docs, OpenAPI, server, snapshots, and SDK.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
