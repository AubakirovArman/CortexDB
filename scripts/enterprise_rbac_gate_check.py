#!/usr/bin/env python3
"""Validate Enterprise RBAC local evidence gates."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


GATE_MARKERS = {
    "rbac-policy-store": {
        "schema": "cortexdb.enterprise_rbac.rbac_policy_store_gate.v1",
        "markers": [
            ("docs/AUTH.md", "CORTEXDB_AUTH_POLICY_STORE_FILE"),
            ("docs/RBAC_POLICY_STORE_DESIGN.md", "policy store"),
            ("docs/SECURITY_HARDENING_EVIDENCE.md", "persisted_auth_policy_store: true"),
            ("crates/cortex-server/src/auth.rs", "cortexdb.auth_policy.v1"),
            (
                "crates/cortex-server/src/tests/auth_policy_tests.rs",
                "auth_policy_store_allows_active_principal_and_denies_disabled",
            ),
            (
                "crates/cortex-server/src/tests/auth_policy_tests.rs",
                "auth_policy_store_invalid_json_fails_closed",
            ),
            (
                "crates/cortex-cli/src/cli_auth_review_tests.rs",
                "auth_review_redacts_policy_store_tokens",
            ),
            (
                "crates/cortex-server/src/tests/auth_policy_tests.rs",
                "admin_can_upsert_policy_store_principal",
            ),
            (
                "crates/cortex-server/src/tests/auth_policy_tests.rs",
                "data_token_cannot_mutate_policy_store",
            ),
            (
                "crates/cortex-server/src/tests/auth_policy_tests.rs",
                "admin_can_disable_policy_store_principal_and_rollback",
            ),
        ],
    },
    "quota-policy": {
        "schema": "cortexdb.enterprise_rbac.quota_policy_gate.v1",
        "markers": [
            ("docs/AUTH.md", "request_quota_per_minute"),
            ("docs/SECURITY_HARDENING_EVIDENCE.md", "per_principal_quota: true"),
            ("crates/cortex-server/src/auth.rs", "request_quota_per_minute"),
            (
                "crates/cortex-server/src/tests/security_quota_tests.rs",
                "policy_store_principal_quota_is_isolated_per_principal",
            ),
            (
                "crates/cortex-server/src/tests/security_tests.rs",
                "rate_limit_returns_typed_429_when_enabled",
            ),
            (
                "crates/cortex-cli/src/cli_auth_review_tests.rs",
                "auth_review_rejects_zero_quota",
            ),
        ],
    },
    "audit-chain": {
        "schema": "cortexdb.enterprise_rbac.audit_chain_gate.v1",
        "markers": [
            ("docs/AUTH.md", "cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain"),
            ("docs/SECURITY_HARDENING_EVIDENCE.md", "audit_chain_foundation: true"),
            ("crates/cortex-server/src/audit.rs", "prev_hash"),
            ("crates/cortex-server/src/audit_tests.rs", "audit_sink_continues_chain_when_reopened"),
            (
                "crates/cortex-cli/src/cli_audit_tests.rs",
                "audit_review_verify_chain_accepts_valid_sequence_and_rejects_tampering",
            ),
            ("crates/cortex-cli/src/cli_audit_tests.rs", "audit_command_can_verify_chain"),
        ],
    },
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate(gate: str) -> dict[str, object]:
    spec = GATE_MARKERS.get(gate)
    if spec is None:
        raise RuntimeError(f"unknown gate {gate!r}")

    failures: list[str] = []
    checked_files: list[str] = []
    markers = spec["markers"]
    assert isinstance(markers, list)
    for file_name, marker in markers:
        checked_files.append(file_name)
        if marker not in read(Path(file_name)):
            failures.append(f"{file_name}: missing marker {marker!r}")

    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "checked_files": sorted(set(checked_files)),
        "failures": failures,
        "boundary": "local Enterprise RBAC evidence gate; not external compliance certification",
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATE_MARKERS))
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(args.gate)
    except RuntimeError as error:
        print(f"enterprise RBAC gate check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1

    print(f"enterprise RBAC {args.gate} check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
