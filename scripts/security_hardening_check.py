#!/usr/bin/env python3
"""Validate Core Alpha security hardening evidence and boundaries."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "persisted_auth_policy_store": [
        ("docs/AUTH.md", "CORTEXDB_AUTH_POLICY_STORE_FILE"),
        ("docs/SECURITY_BETA_BASELINE.md", "implemented JSON principal policy store"),
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "JSON principal policy store"),
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "auth_policy_store_allows_active_principal_and_denies_disabled"),
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "auth_policy_store_invalid_json_fails_closed"),
    ],
    "per_token_quota_boundary": [
        ("docs/SECURITY_BETA_BASELINE.md", "Key quota counters by safe token fingerprint"),
        ("docs/SECURITY_THREAT_MODEL.md", "Per-token quotas"),
        ("crates/cortex-server/src/tests/security_tests.rs", "rate_limit_returns_typed_429_when_enabled"),
    ],
    "audit_principal_metadata": [
        ("docs/AUTH.md", "principal_id`, `auth_role`, and `auth_agent_id`"),
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "Principal-aware audit metadata"),
        ("crates/cortex-server/src/tests/security_tests.rs", "audit_log_file_records_policy_store_principal_without_token"),
    ],
    "audit_chain_foundation": [
        ("docs/AUTH.md", "cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain"),
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "audit_chain_foundation: true"),
        ("crates/cortex-server/src/audit_tests.rs", "audit_sink_continues_chain_when_reopened"),
        ("crates/cortex-cli/src/cli_audit_tests.rs", "audit_review_verify_chain_accepts_valid_sequence_and_rejects_tampering"),
        ("crates/cortex-cli/src/cli_audit_tests.rs", "audit_command_can_verify_chain"),
    ],
    "audit_redaction": [
        ("crates/cortex-server/src/tests/security_tests.rs", "audit_log_file_redacts_ingestion_query_and_body"),
        ("crates/cortex-cli/src/cli_audit_tests.rs", "redaction_ok=true"),
    ],
    "tamper_evident_audit_boundary": [
        ("docs/SECURITY_BETA_BASELINE.md", "Tamper-Evident Audit Chain"),
        ("docs/SECURITY_THREAT_MODEL.md", "Compliance-grade audit trails or SIEM export"),
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "compliance-grade ledger/SIEM export remains future work"),
    ],
    "encrypted_backup_boundary": [
        ("docs/SECURITY_BETA_BASELINE.md", "encrypted backup restore drill succeeds"),
        ("docs/ENCRYPTED_BACKUPS_DESIGN.md", "The current repository does not implement encrypted backups yet"),
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "current backup/restore/offsite staging are local and unencrypted"),
    ],
    "remote_backup_boundary": [
        ("docs/FUTURE_PRODUCT_LAYERS_PLAN.md", "Remote object-store upload"),
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "provider-backed object-store upload remains future work"),
    ],
    "secret_rotation_docs": [
        ("docs/AUTH.md", "CORTEXDB_AUTH_TOKENS_FILE"),
        ("docs/SECURITY_RELEASE_CHECKLIST.md", "local token rotation"),
    ],
    "dashboard_auth_hardening": [
        ("docs/SECURITY_BETA_BASELINE.md", "data tokens cannot access dashboard HTML or assets"),
        ("crates/cortex-server/src/tests/auth_policy_tests.rs", "data_token_cannot_access_dashboard"),
        ("docs/SECURITY_RELEASE_CHECKLIST.md", "Dashboard access is treated as administrative"),
    ],
    "malicious_ingestion_tests": [
        ("crates/cortex-server/src/tests/security_tests.rs", "malicious_ingestion_scope_bypass_is_denied_by_agent_view"),
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "denied_ingestion_audit_event_does_not_leak_query_body_or_token"),
    ],
    "security_beta_baseline": [
        ("docs/SECURITY_BETA_BASELINE.md", "Release Blocking Rule"),
        ("docs/SECURITY_BETA_BASELINE.md", "implemented today from controls that remain"),
    ],
    "security_release_checklist": [
        ("docs/SECURITY_RELEASE_CHECKLIST.md", "Required Local Gates"),
        ("docs/SECURITY_RELEASE_CHECKLIST.md", "Explicit Non-Goals For Core Alpha"),
    ],
    "production_candidate_decisions": [
        ("docs/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "Decision Matrix"),
        ("docs/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "Release-blocking rule"),
        ("docs/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "enterprise RBAC"),
        ("docs/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "built-in encrypted backups"),
        ("docs/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "remote object-store backups"),
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
        print(f"security hardening check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"security hardening check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
