#!/usr/bin/env python3
"""Create the beta security gate report after focused security tests pass."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


CHECKS = {
    "auth_required": [
        ("crates/cortex-server/src/tests/security_tests.rs", "v1_api_requires_bearer_token_when_configured"),
        ("crates/cortex-server/src/tests/error_taxonomy_tests.rs", '"unauthorized"'),
    ],
    "wrong_token_rejected": [
        ("crates/cortex-server/src/tests/security_tests.rs", "v1_api_rejects_wrong_bearer_token_when_configured"),
        ("crates/cortex-server/src/tests/security_tests.rs", "Bearer wrong-secret"),
        ("crates/cortex-server/src/tests/error_taxonomy_tests.rs", "ErrorCode::Unauthorized"),
    ],
    "data_token_admin_denied": [
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "data_token_cannot_access_admin_routes"),
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "data_token_cannot_access_dashboard"),
    ],
    "tenant_traversal_rejected": [
        ("crates/cortex-server/src/tests/security_tests.rs", "test_tenant_path_traversal_over_http"),
        ("crates/cortex-server/src/tests/security_tests.rs", "../../escape"),
    ],
    "rate_limit_works": [
        ("crates/cortex-server/src/tests/security_tests.rs", "rate_limit_returns_typed_429_when_enabled"),
        ("crates/cortex-server/src/tests/security_quota_tests.rs", "policy_store_principal_quota_is_isolated_per_principal"),
    ],
    "cors_allowlist_works": [
        ("crates/cortex-server/src/tests/security_tests.rs", "cors_preflight_is_only_enabled_for_configured_origin"),
        ("crates/cortex-server/src/tests/security_tests.rs", "https://app.example"),
    ],
    "audit_redacts_body_query_token": [
        ("crates/cortex-server/src/tests/security_tests.rs", "audit_log_file_redacts_ingestion_query_and_body"),
        ("crates/cortex-server/src/tests/security_tests.rs", "audit_log_file_records_policy_store_principal_without_token"),
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "denied_ingestion_audit_event_does_not_leak_query_body_or_token"),
    ],
    "agent_view_scope_enforcement": [
        ("crates/cortex-server/src/tests/security_tests.rs", "auth_agent_view_blocks_unreadable_scope_over_http"),
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "token_policy_agent_id_applies_agent_view_scope"),
    ],
    "body_limit_works": [
        ("crates/cortex-server/src/tests/security_tests.rs", "test_server_concurrency_and_size_limit"),
        ("crates/cortex-server/src/tests/security_tests.rs", "413 Payload Too Large"),
        ("crates/cortex-server/src/lib.rs", "RequestBodyLimitLayer"),
    ],
    "openapi_contract_gate": [
        ("Makefile", "openapi-contract-check"),
        ("docs/openapi.yaml", "401"),
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
    for name, markers in CHECKS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return {
        "schema_version": "cortexdb.security_beta_gate.v1",
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "checks": checks,
        "boundary": {
            "proves": "local beta security controls are covered by focused tests and docs",
            "does_not_prove": "enterprise compliance, external identity, or managed-cloud security",
        },
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
        print(f"security beta check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"security beta check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
