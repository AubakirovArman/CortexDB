#!/usr/bin/env python3
"""Aggregate Phase 2 crypto foundation evidence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


SUCCESS_STATUSES = {"passed", "ok"}

REPORT_ARGS = {
    "crypto_deps_policy": "crypto_deps_policy_report",
    "crypto_primitives": "crypto_primitives_report",
    "encrypted_backup": "encrypted_backup_report",
    "encrypted_backup_legacy_refuse": "encrypted_backup_legacy_refuse_report",
    "audit_chain": "audit_chain_report",
    "audit_receipt_binding": "audit_receipt_binding_report",
    "key_management": "key_management_report",
    "database_instance_identity": "database_instance_identity_report",
    "llm_secrets": "llm_secrets_report",
    "secrets_hygiene": "secrets_hygiene_report",
    "crypto_claims_honesty": "crypto_claims_honesty_report",
}

REQUIRED_MAKE_TERMS = [
    "CRYPTO_FOUNDATION_REPORT ?= target/crypto-foundation/report.json",
    "crypto-foundation-check:",
    "$(MAKE) crypto-deps-policy-check",
    "$(MAKE) crypto-primitives-check",
    "$(MAKE) encrypted-backup-check",
    "$(MAKE) encrypted-backup-legacy-refuse-check",
    "$(MAKE) audit-chain-check",
    "$(MAKE) audit-receipt-binding-check",
    "$(MAKE) key-management-check",
    "$(MAKE) database-instance-identity-check",
    "$(MAKE) secrets-check",
    "$(MAKE) crypto-claims-honesty-check",
    'python3 scripts/crypto_foundation_check.py --root "." --crypto-deps-policy-report "$(CRYPTO_DEPS_POLICY_REPORT)"',
    '--audit-receipt-binding-report "$(AUDIT_RECEIPT_BINDING_REPORT)"',
    '--database-instance-identity-report "$(DATABASE_INSTANCE_IDENTITY_REPORT)"',
    "security-gate-v2-check: security-hardening-check crypto-foundation-check",
    '--crypto-foundation-report "$(CRYPTO_FOUNDATION_REPORT)"',
]

DEFERRED_EXTERNAL_TRANSPARENCY_SERVICE = {
    "gate": "transparency-slo-check",
    "status": "deferred",
    "reason": "transparency-availability-check, transparency-gossip-check, and transparency-slo-check prove CI-safe public monitor availability, uptime, fanout, and continuous operations/SLO evidence; live production deployment and key custody remain separate gates",
}


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"failed to parse {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def report_passed(report: dict[str, Any]) -> bool:
    return report.get("status") in SUCCESS_STATUSES


def production_safe(report: dict[str, Any]) -> bool:
    value = report.get("production_safe")
    return value is not False


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def makefiles_text(root: Path) -> str:
    mk = root / "mk"
    return "\n".join(path.read_text(encoding="utf-8") for path in sorted(mk.glob("*.mk")))


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = Path(args.root).resolve()
    report_paths = {
        name: Path(getattr(args, arg_name)) for name, arg_name in REPORT_ARGS.items()
    }
    reports = {name: read_json(path) for name, path in report_paths.items()}

    failures: list[str] = []
    component_status: dict[str, str] = {}
    for name, report in reports.items():
        status = str(report.get("status"))
        component_status[name] = status
        if not report_passed(report):
            failures.append(f"{name}: report status is not a success value: {status}")
        if not production_safe(report):
            failures.append(f"{name}: production_safe is false")

    wiring_text = makefiles_text(root)
    wiring_text += "\n" + (root / "mk/phony.mk").read_text(encoding="utf-8")
    failures.extend(missing_terms("crypto foundation make wiring", wiring_text, REQUIRED_MAKE_TERMS))

    return {
        "schema_version": "cortexdb.crypto_foundation.report.v1",
        "status": "passed" if not failures else "failed",
        "production_safe": not failures,
        "component_status": component_status,
        "reports": {name: str(path) for name, path in report_paths.items()},
        "deferred": [DEFERRED_EXTERNAL_TRANSPARENCY_SERVICE],
        "checked": {
            "success_statuses": sorted(SUCCESS_STATUSES),
            "make_terms": REQUIRED_MAKE_TERMS,
        },
        "failures": failures,
        "boundary": {
            "proves": "implemented single-node crypto foundation gates for dependency policy, shared primitives, AEAD backup, keyed audit chain, receipt-hash audit binding, key custody, durable database-instance identity, secrets hygiene, and public claim honesty",
            "does_not_prove": "external witnessed transparency, KMS/HSM custody, or compliance-grade immutability",
        },
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--crypto-deps-policy-report", required=True)
    parser.add_argument("--crypto-primitives-report", required=True)
    parser.add_argument("--encrypted-backup-report", required=True)
    parser.add_argument("--encrypted-backup-legacy-refuse-report", required=True)
    parser.add_argument("--audit-chain-report", required=True)
    parser.add_argument("--audit-receipt-binding-report", required=True)
    parser.add_argument("--key-management-report", required=True)
    parser.add_argument("--database-instance-identity-report", required=True)
    parser.add_argument("--llm-secrets-report", required=True)
    parser.add_argument("--secrets-hygiene-report", required=True)
    parser.add_argument("--crypto-claims-honesty-report", required=True)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except RuntimeError as error:
        print(f"crypto foundation check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"crypto foundation check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
