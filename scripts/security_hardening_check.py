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
        ("docs/archive/SECURITY_BETA_BASELINE.md", "implemented JSON principal policy store"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "JSON principal policy store"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "rbac_policy_store_gate: true"),
        ("Makefile", "rbac-policy-store-check"),
        ("scripts/enterprise_rbac_gate_check.py", "cortexdb.enterprise_rbac.rbac_policy_store_gate.v1"),
        ("crates/cortex-server/src/tests/auth_policy_tests/store_validation.rs", "auth_policy_store_allows_active_principal_and_denies_disabled"),
        ("crates/cortex-server/src/tests/auth_policy_tests/store_validation.rs", "auth_policy_store_invalid_json_fails_closed"),
        ("crates/cortex-server/src/tests/auth_policy_tests/admin_principals.rs", "admin_can_upsert_policy_store_principal"),
        ("crates/cortex-server/src/tests/auth_policy_tests/admin_principals.rs", "admin_can_disable_policy_store_principal_and_rollback"),
        ("crates/cortex-server/src/tests/auth_policy_tests/store_validation.rs", "auth_policy_store_capabilities_restrict_data_routes"),
        ("crates/cortex-cli/src/cli_auth_review_tests.rs", "auth_review_rejects_invalid_capability"),
        ("docs/archive/RBAC_POLICY_STORE_DESIGN.md", "Action capability restrictions"),
    ],
    "auth_policy_review": [
        ("docs/AUTH.md", "cortexdb auth-review --policy-store"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "auth_policy_review: true"),
        ("crates/cortex-cli/src/cli_auth_review.rs", "cortexdb.auth_review.v1"),
        ("crates/cortex-cli/src/cli_auth_review_tests.rs", "auth_review_redacts_policy_store_tokens"),
    ],
    "per_token_quota_boundary": [
        ("docs/archive/SECURITY_BETA_BASELINE.md", "request_quota_per_minute"),
        ("docs/archive/SECURITY_THREAT_MODEL.md", "Per-token quotas by route class"),
        ("crates/cortex-server/src/tests/security_tests/http_controls.rs", "rate_limit_returns_typed_429_when_enabled"),
    ],
    "per_principal_quota": [
        ("docs/AUTH.md", "request_quota_per_minute"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "per_principal_quota: true"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "quota_policy_gate: true"),
        ("Makefile", "quota-policy-check"),
        ("scripts/enterprise_rbac_gate_check.py", "cortexdb.enterprise_rbac.quota_policy_gate.v1"),
        ("crates/cortex-server/src/auth/tests.rs", "auth_policy_store_rejects_zero_quota"),
        ("crates/cortex-server/src/tests/security_quota_tests.rs", "policy_store_principal_quota_is_isolated_per_principal"),
    ],
    "audit_principal_metadata": [
        ("docs/AUTH.md", "principal_id`, `auth_role`, and `auth_agent_id`"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "Principal-aware audit metadata"),
        ("crates/cortex-server/src/tests/security_tests/audit.rs", "audit_log_file_records_policy_store_principal_without_token"),
    ],
    "audit_chain_foundation": [
        ("docs/AUTH.md", "cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "audit_chain_foundation: true"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "audit_chain_gate: true"),
        ("Makefile", "audit-chain-check"),
        ("scripts/enterprise_rbac_gate_check.py", "cortexdb.enterprise_rbac.audit_chain_gate.v1"),
        ("crates/cortex-server/src/audit_tests.rs", "audit_sink_continues_chain_when_reopened"),
        ("crates/cortex-cli/src/cli_audit_tests.rs", "audit_review_verify_chain_accepts_valid_sequence_and_rejects_tampering"),
        ("crates/cortex-cli/src/cli_audit_tests.rs", "audit_command_can_verify_chain"),
    ],
    "siem_audit_export": [
        ("docs/AUTH.md", "audit-export-siem"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "siem_audit_export: true"),
        ("crates/cortex-cli/src/cli_audit_siem.rs", "cortexdb.siem.audit.v1"),
        ("crates/cortex-cli/src/cli_audit_siem_tests.rs", "audit_export_siem_writes_normalized_jsonl"),
        ("crates/cortex-cli/src/cli_audit_siem_tests.rs", "audit_export_siem_rejects_redaction_violations"),
        ("crates/cortex-cli/src/cli_audit_siem_tests.rs", "audit_export_siem_rejects_chain_violations"),
    ],
    "audit_export_retention_policy": [
        ("docs/archive/AUDIT_EXPORT_RETENTION_POLICY.md", "Audit Export And Retention Policy"),
        ("docs/AUDIT_EXPORT_RETENTION_POLICY.json", "cortexdb.audit_export_retention_policy.v1"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "audit_export_retention_gate: true"),
        ("Makefile", "audit-export-retention-check"),
        ("scripts/audit_export_retention_check.py", "cortexdb.audit_export_retention_report.v1"),
    ],
    "audit_productization": [
        ("docs/AUDIT_LOG_FORMAT.md", "cortexdb.audit.v2"),
        ("docs/AUDIT_LOG_FORMAT.md", "CORTEXDB_AUDIT_MAC_KEY_HEX"),
        ("docs/AUDIT_LOG_FORMAT.md", "--mac-key-file"),
        ("docs/AUDIT_LOG_FORMAT.md", "CORTEXDB_AUDIT_LOG_ROTATE_BYTES"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "audit_productization_gate: true"),
        ("Makefile", "audit-productization-check"),
        ("scripts/audit_productization_check.py", "cortexdb.audit_productization_report.v1"),
    ],
    "compliance_boundary_mapping": [
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb.compliance_boundary.v1"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "Supported certified frameworks today: none."),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "compliance_boundary_mapping: true"),
        ("Makefile", "compliance-boundary-check"),
    ],
    "audit_redaction": [
        ("crates/cortex-server/src/tests/security_tests/audit.rs", "audit_log_file_redacts_ingestion_query_and_body"),
        ("crates/cortex-cli/src/cli_audit_tests.rs", "redaction_ok=true"),
    ],
    "tamper_evident_audit_boundary": [
        ("docs/archive/SECURITY_BETA_BASELINE.md", "Tamper-Evident Audit Chain"),
        ("docs/archive/SECURITY_THREAT_MODEL.md", "Compliance-grade audit trails or vendor-managed SIEM delivery"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "compliance-grade ledger and vendor-managed SIEM delivery remain future work"),
    ],
    "encrypted_backup_boundary": [
        ("docs/archive/SECURITY_BETA_BASELINE.md", "encrypted backup restore drill succeeds"),
        ("docs/archive/ENCRYPTED_BACKUPS_DESIGN.md", "local MVP implemented"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "passphrase encrypted backup MVP"),
    ],
    "remote_backup_boundary": [
        ("docs/FUTURE_PRODUCT_LAYERS_PLAN.md", "Remote object-store upload"),
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "provider-backed object-store upload remains future work"),
    ],
    "secret_rotation_docs": [
        ("docs/AUTH.md", "CORTEXDB_AUTH_TOKENS_FILE"),
        ("docs/archive/SECURITY_RELEASE_CHECKLIST.md", "local token rotation"),
    ],
    "dashboard_auth_hardening": [
        ("docs/archive/SECURITY_BETA_BASELINE.md", "data tokens cannot access dashboard HTML or assets"),
        ("crates/cortex-server/src/tests/auth_policy_tests/role_routes.rs", "data_token_cannot_access_dashboard"),
        ("docs/archive/SECURITY_RELEASE_CHECKLIST.md", "Dashboard access is treated as administrative"),
    ],
    "malicious_ingestion_tests": [
        ("crates/cortex-server/src/tests/security_tests/auth.rs", "malicious_ingestion_scope_bypass_is_denied_by_agent_view"),
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "denied_ingestion_audit_event_does_not_leak_query_body_or_token"),
    ],
    "security_beta_baseline": [
        ("docs/archive/SECURITY_BETA_BASELINE.md", "Release Blocking Rule"),
        ("docs/archive/SECURITY_BETA_BASELINE.md", "implemented today from controls that remain"),
    ],
    "security_release_checklist": [
        ("docs/archive/SECURITY_RELEASE_CHECKLIST.md", "Required Local Gates"),
        ("docs/archive/SECURITY_RELEASE_CHECKLIST.md", "Explicit Non-Goals For Core Alpha"),
    ],
    "production_candidate_decisions": [
        ("docs/archive/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "Decision Matrix"),
        ("docs/archive/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "Release-blocking rule"),
        ("docs/archive/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "enterprise RBAC"),
        ("docs/archive/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "passphrase encrypted backup archives"),
        ("docs/archive/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md", "remote object-store backups"),
    ],
}


def read(path: Path) -> str:
    if str(path) == "Makefile":
        return "\n".join(
            part.read_text(encoding="utf-8")
            for part in [Path("Makefile"), *sorted(Path("mk").glob("*.mk"))]
        )
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
