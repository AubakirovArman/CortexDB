#!/usr/bin/env python3
"""Validate the DV2 integer-only numeric normalization gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-engine/src/verification/numeric/value.rs": [
        "NumericComparison::CurrencyMismatch",
        "pub fn is_conflict",
        "fn normalize_unit_value",
        "UnitClass::Length",
        "UnitClass::Mass",
        "UnitClass::Time",
        "u128::from(value) * multiplier",
    ],
    "crates/cortex-engine/src/verification/numeric/parse.rs": [
        '"meter" | "meters"',
        '"minute" | "minutes"',
        '"hour" | "hours"',
        '"second" | "seconds"',
        '"gram" | "grams"',
    ],
    "crates/cortex-engine/src/verification/numeric/tests.rs": [
        "compatible_units_compare_in_integer_base_units",
        "unit_aliases_normalize_to_same_base_units",
        "cross_currency_conflict_is_labeled",
        "large_unit_conversion_uses_u128_without_overflow",
    ],
    "crates/cortex-engine/src/context/conflicts.rs": [
        ".is_conflict()",
    ],
    "mk/core-retrieval-context.mk": [
        "verify-numeric-normalization-check:",
        'python3 scripts/verify_numeric_normalization_check.py --root "." --report "$(VERIFY_NUMERIC_NORMALIZATION_REPORT)"',
        "$(MAKE) verify-numeric-normalization-check",
    ],
    "mk/vars-core.mk": [
        "VERIFY_NUMERIC_NORMALIZATION_REPORT ?= target/verification-quality/numeric-normalization-report.json",
    ],
    "mk/phony.mk": [
        "verify-numeric-normalization-check",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "DV2",
        "verify-numeric-normalization-check",
        "CurrencyMismatch",
    ],
}

FORBIDDEN_TERMS = {
    "crates/cortex-engine/src/verification/numeric/value.rs": ["f32", "f64"],
    "crates/cortex-engine/src/verification/numeric/parse.rs": ["f32", "f64"],
    "crates/cortex-engine/src/context/conflicts.rs": ["f32", "f64"],
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
    checked: dict[str, Any] = {
        "required_terms": REQUIRED_TERMS,
        "forbidden_terms": FORBIDDEN_TERMS,
    }
    for rel, terms in REQUIRED_TERMS.items():
        try:
            text = read_text(root, rel)
        except FileNotFoundError:
            failures.append(f"{rel}: missing file")
            continue
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing numeric-normalization marker: {term}")

    for rel, terms in FORBIDDEN_TERMS.items():
        try:
            text = read_text(root, rel)
        except FileNotFoundError:
            failures.append(f"{rel}: missing file")
            continue
        for term in terms:
            if term in text:
                failures.append(f"{rel}: forbidden floating-point marker: {term}")

    return {
        "schema_version": "cortexdb.verify_numeric_normalization.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": checked,
        "invariant": (
            "numeric comparison uses integer-only unit-class normalization, "
            "labels cross-currency conflicts, and avoids f32/f64"
        ),
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
    print(f"verify numeric normalization check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
