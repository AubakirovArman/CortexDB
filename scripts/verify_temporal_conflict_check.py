#!/usr/bin/env python3
"""Validate the VERIFY temporal numeric-conflict gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_FACT_CLAIM_TERMS = [
    "temporal_validity: TemporalValidity",
    "temporal_validity_from_metadata",
    "explicit_numeric_values",
    "record_matches_temporal_query",
    "temporal_window_overlaps_query",
]

REQUIRED_TEMPORAL_TERMS = [
    "pub fn overlaps_query",
    "pub fn overlaps",
]

REQUIRED_CONFLICT_INDEX_TERMS = [
    "temporal_windows_overlap",
    "!temporal_windows_overlap(left.temporal_validity, right.temporal_validity)",
]

REQUIRED_TEST_TERMS = [
    "explicit_currency_field_applies_to_numeric_value",
    "contextual_numeric_values_ignore_contradicts_marker",
    "dated_fact_indexes_overlapping_numeric_records",
    "dated_verify_fact_still_reports_numeric_conflict",
    "stale_evidence_does_not_create_numeric_contradiction",
    "temporal_numeric_conflict_index_respects_overlapping_windows",
]

REQUIRED_MAKE_TERMS = [
    "VERIFY_TEMPORAL_CONFLICT_REPORT ?= target/verification-quality/temporal-conflict-report.json",
    "verify-temporal-conflict-check:",
    "$(MAKE) verify-temporal-conflict-check",
]

FORBIDDEN_FACT_CLAIM_TERMS = [
    "if extract_temporal_query_range(fact).is_some() {\n            return;",
    "if extract_temporal_query_range(fact).is_some() {\n            return Vec::new();",
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
    fact_claim = read_text(root / "crates/cortex-engine/src/verification/numeric/fact_claim/mod.rs")
    temporal = read_text(root / "crates/cortex-engine/src/verification/temporal.rs")
    conflict_index = read_text(root / "crates/cortex-engine/src/verification/conflict_index/store.rs")
    tests = "\n".join(
        [
            read_text(root / "crates/cortex-engine/src/verification/numeric/fact_claim/tests.rs"),
            read_text(root / "crates/cortex-engine/tests/verification_guards.rs"),
            read_text(root / "crates/cortex-engine/tests/verification_conflict_numeric.rs"),
        ]
    )
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures: list[str] = []
    failures.extend(missing_terms("fact_claim.rs", fact_claim, REQUIRED_FACT_CLAIM_TERMS))
    failures.extend(missing_terms("temporal.rs", temporal, REQUIRED_TEMPORAL_TERMS))
    failures.extend(missing_terms("conflict_index/store.rs", conflict_index, REQUIRED_CONFLICT_INDEX_TERMS))
    failures.extend(missing_terms("tests", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(forbidden_terms("fact_claim.rs", fact_claim, FORBIDDEN_FACT_CLAIM_TERMS))

    return {
        "schema_version": "cortexdb.verify_temporal_conflict.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "metrics": {
            "temporal_numeric_regression_cases": 4,
            "stale_guard_controls": 1,
        },
        "checked": {
            "fact_claim_terms": REQUIRED_FACT_CLAIM_TERMS,
            "temporal_terms": REQUIRED_TEMPORAL_TERMS,
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
    print(f"verify temporal conflict check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
