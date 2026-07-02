#!/usr/bin/env python3
"""Validate the AR-5 accountability receipt signed header gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ACCOUNTABILITY_FILES = [
    "crates/cortex-engine/src/accountability.rs",
    "crates/cortex-engine/src/accountability/receipt.rs",
    "crates/cortex-engine/src/accountability/receipt_leaves.rs",
    "crates/cortex-engine/src/accountability/receipt_tests.rs",
    "crates/cortex-engine/src/accountability/receipt_header.rs",
    "crates/cortex-engine/src/accountability/receipt_sign_tests.rs",
]

REQUIRED_SIGN_TERMS = [
    "AccountabilityReceiptHeader",
    "AccountabilityReceiptSignature",
    "sign_accountability_receipt_header",
    "verify_accountability_receipt_header",
    "canonical_accountability_receipt_header_bytes",
    "accountability_receipt_header_value",
    "audit_chain_head",
    "ReceiptSigningKey",
    "ReceiptKeyRing",
    "ReceiptSignature::from_hex",
    "RECEIPT_SIGNING_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION",
    "ACCOUNTABILITY_RECEIPT_HASH_ALG",
    "ACCOUNTABILITY_RECEIPT_SIG_ALG",
]

REQUIRED_TEST_TERMS = [
    "accountability_receipt_header_signature_is_deterministic",
    "accountability_receipt_header_is_replica_invariant_for_same_committed_inputs",
    "accountability_receipt_header_signature_changes_when_root_changes",
    "accountability_receipt_header_changes_when_audit_chain_head_changes",
    "accountability_receipt_header_verifies_with_trusted_keyring",
    "accountability_receipt_header_rejects_rotated_key_id_and_public_key_mismatch",
]

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_RECEIPT_SIGN_REPORT ?= target/accountability-receipt/sign-report.json",
    "accountability-receipt-sign-check:",
    "cargo test -p cortex-engine accountability_receipt_header --all-features",
    'python3 scripts/accountability_receipt_sign_check.py --root "." --report "$(ACCOUNTABILITY_RECEIPT_SIGN_REPORT)"',
]

FORBIDDEN_SIGN_TERMS = [
    "Instant::now",
    "SystemTime::now",
    "DefaultHasher",
    "HashMap",
    "FNV",
    "fnv",
    "elapsed_nanos",
    "total_elapsed_nanos",
    "CORTEXDB_RECEIPT_SIGNING_KEY_HEX",
    "CORTEXDB_RECEIPT_SIGNING_KEY_FILE",
]

MAX_RUST_FILE_LINES = 300


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden {term}" for term in terms if term in text]


def line_count_failures(root: Path, paths: list[str]) -> list[str]:
    failures = []
    for path in paths:
        line_count = len(read_text(root / path).splitlines())
        if line_count > MAX_RUST_FILE_LINES:
            failures.append(
                f"{path}: {line_count} lines exceeds {MAX_RUST_FILE_LINES} line bound"
            )
    return failures


def validate(root: Path) -> dict[str, Any]:
    accountability_text = "\n".join(read_text(root / path) for path in ACCOUNTABILITY_FILES)
    sign_tests = read_text(root / "crates/cortex-engine/src/accountability/receipt_sign_tests.rs")
    sign_code = read_text(root / "crates/cortex-engine/src/accountability/receipt_header.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")

    failures: list[str] = []
    failures.extend(missing_terms("accountability sign code", accountability_text, REQUIRED_SIGN_TERMS))
    failures.extend(missing_terms("accountability sign tests", sign_tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", phony, ["accountability-receipt-sign-check"]))
    failures.extend(forbidden_terms("accountability signed header code", sign_code, FORBIDDEN_SIGN_TERMS))
    failures.extend(line_count_failures(root, ACCOUNTABILITY_FILES))

    return {
        "schema_version": "cortexdb.accountability_receipt_sign.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "accountability_files": ACCOUNTABILITY_FILES,
            "sign_terms": REQUIRED_SIGN_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_terms": FORBIDDEN_SIGN_TERMS,
            "max_rust_file_lines": MAX_RUST_FILE_LINES,
        },
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
    print(f"accountability receipt sign check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
