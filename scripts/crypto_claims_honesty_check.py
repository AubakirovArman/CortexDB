#!/usr/bin/env python3
"""Validate CRY-7 crypto claim honesty in current public docs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "docs/SECURITY_MODEL.md": [
        "XChaCha20-Poly1305",
        "Argon2id-derived keys",
        "HMAC-SHA-256",
        "CORTEXDB_AUDIT_MAC_KEY_HEX",
        "Receipt signing key custody",
        "configured local",
        "`accountability_receipt.v1` JSON emission",
        "accountability_receipt_hash",
        "cortexdb.receipt_audit_reanchor.v1",
        "KMS-backed envelope encryption",
        "compliance-certified immutable audit export",
    ],
    "docs/BACKUP_RESTORE.md": [
        "cortexdb.encrypted_backup.v2",
        "cortexdb.xchacha20poly1305-argon2id.v2",
        "XChaCha20-Poly1305",
        "Argon2id-derived key",
        "authenticated header AAD",
        "Legacy encrypted backup v1 archives are refused",
        "not live encryption of the running database directory or WAL",
        "not a KMS-backed",
        "no in-place rewrap operation",
    ],
    "docs/AUTH.md": [
        "cortexdb.audit.v2",
        "HMAC-SHA-256",
        "CORTEXDB_AUDIT_MAC_KEY_HEX",
        "--mac-key-file",
        "accountability_receipt_hash",
        "cortexdb.receipt_audit_reanchor.v1",
        "legacy v1 hash-chain records remain readable",
        "not a compliance-certified audit ledger",
    ],
    "docs/AUDIT_LOG_FORMAT.md": [
        "cortexdb.audit.v2",
        "HMAC-SHA-256",
        "CORTEXDB_AUDIT_MAC_KEY_HEX",
        "--mac-key-file",
        "accountability_receipt_hash",
        "without it, keyed records fail verification",
        "not a compliance-certified immutable ledger",
    ],
    "docs/CLI.md": [
        "cortexdb.audit.v2",
        "--mac-key-file",
        "receipt-key generate",
        "receipt-key rotate",
        "receipt-key verify-reanchor",
        "cortexdb.encrypted_backup.v2",
        "XChaCha20-Poly1305",
        "Argon2id-derived key",
    ],
    "mk/core-security-ops.mk": [
        "crypto-claims-honesty-check:",
        'python3 scripts/crypto_claims_honesty_check.py --root "." --report "$(CRYPTO_CLAIMS_HONESTY_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "CRYPTO_CLAIMS_HONESTY_REPORT ?= target/crypto-claims-honesty/report.json",
    ],
    "mk/phony.mk": [
        "crypto-claims-honesty-check",
    ],
}

FORBIDDEN_TERMS = {
    "docs/SECURITY_MODEL.md": [
        "encrypted at-rest backups and secret management integrations",
        "tamper-evident audit export",
        "signed accountability receipts are shipped",
    ],
    "docs/BACKUP_RESTORE.md": [
        "xor-fnv",
        "xor fnv",
        "FNV keystream",
        "XOR-FNV",
    ],
    "docs/AUTH.md": [
        "XOR-FNV",
        "MAC key material are logged",
    ],
    "docs/AUDIT_LOG_FORMAT.md": [
        "Always `cortexdb.audit.v1` for local server audit records",
        "MAC key material",
    ],
    "docs/CLI.md": [
        "XOR-FNV",
        "--mac-key-hex",
    ],
}


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def contains_marker(text: str, marker: str) -> bool:
    if marker in text:
        return True
    return " ".join(marker.split()) in " ".join(text.split())


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, dict[str, list[str]]] = {}
    for rel, terms in REQUIRED_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["required"] = terms
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing required crypto claim marker: {term}")
    for rel, terms in FORBIDDEN_TERMS.items():
        text = read_text(root, rel)
        lower_text = text.lower()
        checked.setdefault(rel, {})["forbidden"] = terms
        for term in terms:
            if term.lower() in lower_text:
                failures.append(f"{rel}: forbidden or stale crypto claim remains: {term}")
    return {
        "schema_version": "cortexdb.crypto_claims_honesty.report.v1",
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
    print(f"crypto claims honesty check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
