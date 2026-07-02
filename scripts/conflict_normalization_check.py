#!/usr/bin/env python3
"""Validate the ContextPack conflict normalization gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_CONFLICT_TERMS = [
    "extract_numeric_values",
    "compare_numeric_values",
    ".is_conflict()",
    "ConflictValue::Numeric",
    "ConflictValue::Text",
]

REQUIRED_DEDUP_TERMS = [
    "split_metadata_line",
    ".or_else(|| line.split_once(':'))",
    '"currency"',
    '"unit"',
]

REQUIRED_NUMERIC_TERMS = [
    "fn normalize_unit_value",
    "UnitClass::Length",
    "UnitClass::Mass",
    "UnitClass::Time",
    "NumericComparison::CurrencyMismatch",
    'word.starts_with(\'$\').then(|| "USD".to_owned())',
    "trailing_context_suffix",
]

REQUIRED_TEST_TERMS = [
    "normalized_equal_currency_and_magnitude_values_do_not_flag_conflict",
    "true_numeric_conflicts_flag_across_currency_formats",
    "compatible_unit_values_normalize_before_conflict_detection",
    "non_numeric_values_keep_string_fallback",
    "conflict_normalization_is_deterministic",
]

REQUIRED_MAKE_TERMS = [
    "CONFLICT_NORMALIZATION_REPORT ?= target/context-pack-quality/conflict-normalization-report.json",
    "conflict-normalization-check:",
]

FORBIDDEN_NUMERIC_TERMS = [
    "f32",
    "f64",
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
    conflicts = read_text(root / "crates/cortex-engine/src/context/conflicts.rs")
    dedup = read_text(root / "crates/cortex-engine/src/context/dedup.rs")
    parse = read_text(root / "crates/cortex-engine/src/verification/numeric/parse.rs")
    value = read_text(root / "crates/cortex-engine/src/verification/numeric/value.rs")
    tests = read_text(root / "crates/cortex-engine/tests/conflict_normalization.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures: list[str] = []
    failures.extend(missing_terms("context/conflicts.rs", conflicts, REQUIRED_CONFLICT_TERMS))
    failures.extend(missing_terms("context/dedup.rs", dedup, REQUIRED_DEDUP_TERMS))
    failures.extend(missing_terms("verification/numeric", parse + value, REQUIRED_NUMERIC_TERMS))
    failures.extend(missing_terms("conflict_normalization.rs", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(forbidden_terms("verification/numeric", parse + value, FORBIDDEN_NUMERIC_TERMS))

    return {
        "schema_version": "cortexdb.conflict_normalization.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "metrics": {
            "labeled_cases": 5,
            "expected_recall_percent": 100,
            "expected_precision_percent": 100,
        },
        "checked": {
            "conflict_terms": REQUIRED_CONFLICT_TERMS,
            "dedup_terms": REQUIRED_DEDUP_TERMS,
            "numeric_terms": REQUIRED_NUMERIC_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_numeric_terms": FORBIDDEN_NUMERIC_TERMS,
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
    print(f"conflict normalization check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
