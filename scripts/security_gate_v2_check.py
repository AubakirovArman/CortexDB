#!/usr/bin/env python3
"""Aggregate CortexDB security gate v2 evidence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REQUIRED_SECURITY_CHECKS = {
    "auth_required",
    "wrong_token_rejected",
    "data_token_admin_denied",
    "tenant_traversal_rejected",
    "rate_limit_works",
    "cors_allowlist_works",
    "audit_redacts_body_query_token",
    "agent_view_scope_enforcement",
    "body_limit_works",
    "openapi_contract_gate",
}
REQUIRED_HARDENING_CHECKS = {
    "persisted_auth_policy_store",
    "auth_policy_review",
    "per_token_quota_boundary",
    "per_principal_quota",
    "audit_principal_metadata",
    "audit_chain_foundation",
    "siem_audit_export",
    "audit_export_retention_policy",
    "secret_rotation_docs",
    "dashboard_auth_hardening",
    "malicious_ingestion_tests",
    "security_beta_baseline",
    "security_release_checklist",
}


def read_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"failed to parse {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def passed(report: dict[str, Any]) -> bool:
    return report.get("status") == "passed"


def bool_checks(report: dict[str, Any]) -> dict[str, bool]:
    checks = report.get("checks")
    if not isinstance(checks, dict):
        return {}
    return {key: value for key, value in checks.items() if isinstance(value, bool)}


def validate_named_checks(
    label: str, report: dict[str, Any], required: set[str], failures: list[str]
) -> dict[str, bool]:
    checks = bool_checks(report)
    missing = sorted(required - set(checks))
    false_checks = sorted(name for name in required if checks.get(name) is not True)
    if missing:
        failures.append(f"{label}: missing checks: {missing}")
    if false_checks:
        failures.append(f"{label}: checks not passed: {false_checks}")
    return {name: checks.get(name, False) for name in sorted(required)}


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    report_paths = {
        "security": Path(args.security_report),
        "security_hardening": Path(args.security_hardening_report),
        "rbac": Path(args.rbac_report),
        "quota": Path(args.quota_report),
        "audit_chain": Path(args.audit_chain_report),
        "audit_export_retention": Path(args.audit_export_retention_report),
    }
    reports = {name: read_report(path) for name, path in report_paths.items()}

    failures: list[str] = []
    for name, report in reports.items():
        if not passed(report):
            failures.append(f"{name}: report status is not passed")

    security_checks = validate_named_checks(
        "security", reports["security"], REQUIRED_SECURITY_CHECKS, failures
    )
    hardening_checks = validate_named_checks(
        "security_hardening",
        reports["security_hardening"],
        REQUIRED_HARDENING_CHECKS,
        failures,
    )

    component_status = {name: passed(report) for name, report in reports.items()}
    return {
        "schema_version": "cortexdb.security_gate_v2.v1",
        "status": "passed" if not failures else "failed",
        "component_status": component_status,
        "required_security_checks": security_checks,
        "required_hardening_checks": hardening_checks,
        "reports": {name: str(path) for name, path in report_paths.items()},
        "failures": failures,
        "boundary": {
            "proves": "local single-node security gate evidence for auth, RBAC policy store, tenant isolation, CORS, rate limits, audit, malicious ingestion, and OpenAPI contracts",
            "does_not_prove": "external identity, enterprise compliance certification, managed-cloud security, or distributed authorization correctness",
        },
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--security-report", required=True)
    parser.add_argument("--security-hardening-report", required=True)
    parser.add_argument("--rbac-report", required=True)
    parser.add_argument("--quota-report", required=True)
    parser.add_argument("--audit-chain-report", required=True)
    parser.add_argument("--audit-export-retention-report", required=True)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except RuntimeError as error:
        print(f"security gate v2 check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"security gate v2 check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
