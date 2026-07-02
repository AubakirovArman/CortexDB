#!/usr/bin/env python3
"""Validate durable database-instance identity wiring for receipt headers."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-server/src/database_identity.rs": [
        "cortexdb.database_instance_identity.v1",
        "cortexdb.database_instance_identity.json",
        "load_or_create_database_instance_id",
        "resolved.receipt_external_signer.is_some()",
        "OpenOptions::new().write(true).create_new(true)",
        "validate_database_instance_id",
        "dbi_",
        "load_or_create_database_instance_id_reuses_existing_file",
        "load_or_create_database_instance_id_rejects_invalid_file",
    ],
    "crates/cortex-server/src/lifecycle.rs": [
        "prepare_server_options",
        "load_or_create_database_instance_id(root)",
        "options.receipt_external_signer.is_some()",
        "database instance identity is required when receipt signing is configured",
    ],
    "crates/cortex-server/src/receipt.rs": [
        "fn from_options(options: &ServerOptions)",
        "database instance identity is required for receipt signing",
        "validate_database_instance_id(db_instance_id)",
    ],
    "crates/cortex-server/src/tests/api_tests/receipt_identity.rs": [
        "configured_receipts_use_durable_database_instance_id_across_tenants",
        "cortexdb.database_instance_identity.json",
        "assert_eq!(default_id, alpha_id)",
        "assert!(!default_id.starts_with(\"local:\"))",
    ],
    "docs/AUTH.md": [
        "cortexdb.database_instance_identity.v1",
        "cortexdb.database_instance_identity.json",
        "db_instance_id",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.database_instance_identity.v1",
        "durable database-instance identity",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "durable database-instance identity",
        "`cortexdb.database_instance_identity.v1`",
        "database-instance-identity-check",
    ],
    "mk/core-security-ops.mk": [
        "database-instance-identity-check:",
        "cargo test -p cortex-server database_instance_id --all-features",
        'python3 scripts/database_instance_identity_check.py --root "." --report "$(DATABASE_INSTANCE_IDENTITY_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "DATABASE_INSTANCE_IDENTITY_REPORT ?= target/database-instance-identity/report.json",
    ],
    "mk/phony.mk": [
        "database-instance-identity-check",
    ],
}

FORBIDDEN_TERMS = {
    "crates/cortex-server/src/receipt.rs": [
        "format!(\"local:{tenant}\")",
        "local:{tenant}",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "the current configured local emission uses a local tenant-derived `db_instance_id`",
    ],
}


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, dict[str, list[str]]] = {}
    for rel, terms in REQUIRED_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["required"] = terms
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing durable identity marker: {term}")
    for rel, terms in FORBIDDEN_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["forbidden"] = terms
        for term in terms:
            if term in text:
                failures.append(f"{rel}: stale tenant-derived identity marker remains: {term}")
    return {
        "schema_version": "cortexdb.database_instance_identity.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": checked,
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"database instance identity check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
