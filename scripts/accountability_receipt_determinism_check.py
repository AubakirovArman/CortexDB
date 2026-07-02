#!/usr/bin/env python3
"""Validate the AR-4 accountability receipt body determinism gate."""

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
]

REQUIRED_RECEIPT_TERMS = [
    "AccountabilityReceiptBody",
    "AccountabilityReceiptLeaves",
    "AccountabilityDeterminismInput",
    "accountability_receipt_body",
    "ACCOUNTABILITY_RECEIPT_ACCESS_ROOT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_PROVENANCE_ROOT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_CELL_SET_ROOT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_BUDGET_COMMITMENT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_CONFLICT_COMMITMENT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_PACK_ROOT_DOMAIN",
    "ACCOUNTABILITY_RECEIPT_DETERMINISM_DOMAIN",
    "merkle_root",
    "canonical_context_pack_bytes",
    "canonical_verification_report_bytes",
    "retrieved_cell_content_hash",
    "CapturedAccessDenialSet",
    "BTreeMap",
    "blake3_256_domain",
]

REQUIRED_TEST_TERMS = [
    "accountability_receipt_body_roots_are_deterministic_and_schema_aligned",
    "accountability_receipt_body_changes_when_cell_payload_changes",
    "accountability_receipt_body_changes_when_determinism_input_changes",
    "accountability_receipt_body_requires_captured_allowed_access",
]

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_RECEIPT_DETERMINISM_REPORT ?= target/accountability-receipt/determinism-report.json",
    "accountability-receipt-determinism-check:",
    "cargo test -p cortex-engine accountability_receipt_body --all-features",
    'python3 scripts/accountability_receipt_determinism_check.py --root "." --report "$(ACCOUNTABILITY_RECEIPT_DETERMINISM_REPORT)"',
]

FORBIDDEN_RECEIPT_TERMS = [
    "Instant::now",
    "SystemTime::now",
    "DefaultHasher",
    "HashMap",
    "FNV",
    "fnv",
    "elapsed_nanos",
    "total_elapsed_nanos",
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
    tests = read_text(root / "crates/cortex-engine/src/accountability/receipt_tests.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")

    failures: list[str] = []
    failures.extend(missing_terms("accountability receipt code", accountability_text, REQUIRED_RECEIPT_TERMS))
    failures.extend(missing_terms("accountability receipt tests", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", phony, ["accountability-receipt-determinism-check"]))
    failures.extend(forbidden_terms("accountability receipt code", accountability_text, FORBIDDEN_RECEIPT_TERMS))
    failures.extend(line_count_failures(root, ACCOUNTABILITY_FILES))

    return {
        "schema_version": "cortexdb.accountability_receipt_determinism.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "accountability_files": ACCOUNTABILITY_FILES,
            "receipt_terms": REQUIRED_RECEIPT_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_terms": FORBIDDEN_RECEIPT_TERMS,
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
    print(f"accountability receipt determinism check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
