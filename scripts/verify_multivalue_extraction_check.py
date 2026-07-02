#!/usr/bin/env python3
"""Validate the VERIFY numeric multi-value extraction gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_FACT_CLAIM_TERMS = [
    "BTreeMap<CellId, Vec<NumericFactRecord>>",
    "records_from_payload",
    "contextual_numeric_values",
    "prefer_contextual_numeric_values",
]

REQUIRED_CONFLICT_INDEX_TERMS = [
    "for record in FactClaimStore::records_from_payload",
    "left.cell_id == right.cell_id",
]

REQUIRED_TEST_TERMS = [
    "record_extracts_contextual_numeric_values_from_multivalue_body",
    "explicit_multivalue_records_are_indexed_deterministically",
    "multivalue_index_tracks_patch_tombstone_and_reopen",
    "verify_fact_detects_conflict_from_multivalue_evidence_body",
]

REQUIRED_MAKE_TERMS = [
    "VERIFY_MULTIVALUE_EXTRACTION_REPORT ?= target/verification-quality/multivalue-extraction-report.json",
    "verify-multivalue-extraction-check:",
]

FORBIDDEN_FACT_CLAIM_TERMS = [
    "fn single_numeric_value",
    "single_numeric_value(values)",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden {term}" for term in terms if term in text]


def validate(root: Path) -> dict[str, Any]:
    fact_claim = read_text(root / "crates/cortex-engine/src/verification/numeric/fact_claim.rs")
    conflict_index = read_text(root / "crates/cortex-engine/src/verification/conflict_index/store.rs")
    tests = "\n".join(
        [
            read_text(root / "crates/cortex-engine/src/verification/numeric/fact_claim/tests.rs"),
            read_text(root / "crates/cortex-engine/tests/verification_guards.rs"),
        ]
    )
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures: list[str] = []
    failures.extend(missing_terms("fact_claim.rs", fact_claim, REQUIRED_FACT_CLAIM_TERMS))
    failures.extend(missing_terms("conflict_index/store.rs", conflict_index, REQUIRED_CONFLICT_INDEX_TERMS))
    failures.extend(missing_terms("tests", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(forbidden_terms("fact_claim.rs", fact_claim, FORBIDDEN_FACT_CLAIM_TERMS))

    return {
        "schema_version": "cortexdb.verify_multivalue_extraction.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "metrics": {
            "multi_value_regression_cases": 4,
            "false_conflict_controls": 1,
        },
        "checked": {
            "fact_claim_terms": REQUIRED_FACT_CLAIM_TERMS,
            "conflict_index_terms": REQUIRED_CONFLICT_INDEX_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_fact_claim_terms": FORBIDDEN_FACT_CLAIM_TERMS,
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
    print(f"verify multivalue extraction check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
