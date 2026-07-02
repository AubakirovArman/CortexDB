#!/usr/bin/env python3
"""Validate the Phase 2 keyed audit/key-management slice."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-server/src/config.rs": [
        "pub struct AuditMacKey",
        "pub struct ReceiptSigningKey",
        "pub fn from_hex",
        "pub fn from_seed_hex",
        "pub fn key_id",
        "pub fn public_key_hex",
        "pub(crate) fn mac_key",
        'debug_struct("AuditMacKey")',
        'debug_struct("ReceiptSigningKey")',
        '.field("secret", &"redacted")',
        '.field("seed", &"redacted")',
        "audit_log_mac_key: Option<AuditMacKey>",
        "receipt_signing_key: Option<ReceiptSigningKey>",
    ],
    "crates/cortex-server/src/main.rs": [
        "CORTEXDB_AUDIT_MAC_KEY_HEX",
        "CORTEXDB_AUDIT_MAC_KEY_ID",
        "CORTEXDB_RECEIPT_SIGNING_KEY_FILE",
        "CORTEXDB_RECEIPT_SIGNING_KEY_HEX",
        "CORTEXDB_RECEIPT_SIGNING_KEY_ID",
        "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND",
        "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX",
        "audit_log_mac_key_from_env",
        "receipt_signing_key_from_env",
        "receipt_external_signer_from_env",
        "parse_audit_log_mac_key",
        "parse_receipt_signing_key",
        "parse_receipt_external_signer",
        "parse_receipt_signing_key_file_json",
        "is required when CORTEXDB_AUDIT_LOG_FILE is set",
    ],
    "crates/cortex-server/src/receipt_signer.rs": [
        "pub struct ReceiptExternalSigner",
        "cortexdb.receipt_external_sign_request.v1",
        "cortexdb.receipt_external_signature.v1",
        "Command::new(&self.command)",
        "signature verification failed",
    ],
    "crates/cortex-server/src/receipt.rs": [
        "ReceiptSigner",
        "External(ReceiptExternalSigner)",
        "signed_receipt_value_with_signer",
        "set only one receipt signer mode",
    ],
    "crates/cortex-engine/src/accountability/receipt_header.rs": [
        "pub trait AccountabilityReceiptHeaderSigner",
        "sign_accountability_receipt_header_with_signer",
    ],
    "crates/cortex-crypto/src/receipt_key.rs": [
        "RECEIPT_SIGNING_DOMAIN",
        "pub struct ReceiptSigningKey",
        "pub struct ReceiptPublicKey",
        "pub struct ReceiptSignature",
        "pub struct ReceiptKeyRing",
        "receipt_keyring_verifies_current_and_previous_keys",
    ],
    "crates/cortex-server/src/audit.rs": [
        'AUDIT_SCHEMA_VERSION_V2: &str = "cortexdb.audit.v2"',
        "mac_key_id: Option<String>",
        "event_mac: Option<String>",
        "audit_event_mac",
    ],
    "crates/cortex-server/src/audit/sink.rs": [
        "audit log MAC key is required for persisted audit chain v2",
        "record.schema_version = AUDIT_SCHEMA_VERSION_V2",
        "record.mac_key_id = Some",
        "record.event_mac = Some",
    ],
    "crates/cortex-cli/src/cli/args/commands.rs": [
        'long = "mac-key-file"',
        "ReceiptKeyCommand",
    ],
    "crates/cortex-cli/src/cli/args/commands/subcommands.rs": [
        "VerifyReanchor",
        'long = "reanchor-file"',
        'long = "audit-chain-head"',
        'long = "audit-sequence"',
    ],
    "crates/cortex-cli/src/cli_receipt_key.rs": [
        "cortexdb.receipt_signing_key.v1",
        "cortexdb.receipt_public_key.v1",
        "cortexdb.receipt_trust.v1",
        "verify_reanchor",
        "write_trust_file",
        "signing_seed_hex",
    ],
    "crates/cortex-cli/src/cli_receipt_key/reanchor.rs": [
        "cortexdb.receipt_audit_reanchor.v1",
        "cortexdb.receipt_audit_reanchor.hash.v1",
        "cortexdb.receipt_trust.hash.v1",
        "build_reanchor_record",
        "read_and_verify_reanchor",
        "previous_signature_hex",
        "current_signature_hex",
        "audit_chain_head",
        "trust_manifest_hash",
    ],
    "crates/cortex-cli/src/cli_audit_chain.rs": [
        "verify_record_mac",
        "event_mac_for_record",
        "mac_key_from_hex",
        'AUDIT_SCHEMA_VERSION_V2: &str = "cortexdb.audit.v2"',
    ],
    "crates/cortex-cli/src/cli_audit_tests.rs": [
        "audit_review_verify_chain_requires_mac_key_for_v2_and_rejects_mac_tampering",
        "audit_verify_alias_accepts_keyed_v2_chain_with_key_file",
    ],
    "crates/cortex-cli/src/cli_receipt_key_tests.rs": [
        "receipt_key_generate_export_and_rotate_preserves_dual_trust",
        "receipt_key_rotate_writes_verifiable_reanchor_record",
        "verify-reanchor",
        "cortexdb.receipt_audit_reanchor.v1",
        "historical receipt header",
        "current receipt header",
        "signing_seed_hex",
    ],
    "mk/core-security-ops.mk": [
        "key-management-check:",
        "cargo test -p cortex-crypto receipt_key",
        "cargo test -p cortex-server audit_tests",
        "cargo test -p cortex-server parse_receipt_signing_key",
        "cargo test -p cortex-server parse_receipt_external_signer",
        "cargo test -p cortex-server receipt_signer",
        "cargo test -p cortex-cli receipt_key_generate_export_and_rotate_preserves_dual_trust",
        "cargo test -p cortex-cli receipt_key_rotate_writes_verifiable_reanchor_record",
        "cargo test -p cortex-cli audit_review_verify_chain_requires_mac_key_for_v2_and_rejects_mac_tampering",
        'python3 scripts/key_management_check.py --root "." --report "$(KEY_MANAGEMENT_REPORT)"',
    ],
    "docs/AUTH.md": [
        "CORTEXDB_RECEIPT_SIGNING_KEY_FILE",
        "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND",
        "cortexdb.receipt_signing_key.v1",
        "cortexdb.receipt_external_sign_request.v1",
        "cortexdb.receipt_external_signature.v1",
        "cortexdb.receipt_trust.v1",
        "cortexdb.receipt_audit_reanchor.v1",
        "verify-reanchor",
    ],
    "docs/CLI.md": [
        "receipt-key generate",
        "receipt-key export-public",
        "receipt-key rotate",
        "receipt-key verify-reanchor",
        "--reanchor-file",
    ],
    "mk/vars-core.mk": [
        "KEY_MANAGEMENT_REPORT ?= target/key-management/report.json",
    ],
    "mk/phony.mk": [
        "key-management-check",
    ],
}

FORBIDDEN_TERMS = {
    "crates/cortex-cli/src/cli/args/commands.rs": [
        'long = "mac-key-hex"',
        'long = "signing-seed"',
        'long = "signing-seed-hex"',
    ],
}


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
        missing = [term for term in terms if term not in text]
        checked.setdefault(rel, {})["required"] = terms
        failures.extend(f"{rel}: missing {term}" for term in missing)
    for rel, terms in FORBIDDEN_TERMS.items():
        text = read_text(root, rel)
        present = [term for term in terms if term in text]
        checked.setdefault(rel, {})["forbidden"] = terms
        failures.extend(f"{rel}: forbidden direct secret argument remains: {term}" for term in present)
    return {
        "schema_version": "cortexdb.key_management.report.v1",
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
    print(f"key management check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
