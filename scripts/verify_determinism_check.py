#!/usr/bin/env python3
"""Validate DV6 verification determinism gate wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_CANONICAL_TERMS = [
    "canonical_verification_report_bytes",
    "verification_report_value",
    "numeric_conflict_value",
    '"numeric_conflicts": report.numeric_conflicts.iter().map(numeric_conflict_value)',
    '"kind": conflict.kind.as_str()',
    '"cell_id": conflict.cell_id.0',
    '"metric": conflict.metric',
    '"left": conflict.left',
    '"right": conflict.right',
]

REQUIRED_TEST_TERMS = [
    "verification_canonical_conflict_bytes_are_repeatable_and_clock_free",
    "verification_report_canonical_conflict_kind_is_hashed",
    'text.contains(r#""kind":"numeric""#)',
    'text.contains(r#""kind":"citation""#)',
]

REQUIRED_MAKE_TERMS = [
    "VERIFY_DETERMINISM_REPORT ?= target/verification-quality/determinism-report.json",
    "verify-determinism-check:",
    "cargo test -p cortex-engine --test determinism verification_canonical_conflict_bytes_are_repeatable_and_clock_free --all-features",
    "cargo test -p cortex-engine canonical --all-features",
    'python3 scripts/verify_determinism_check.py --root "." --report "$(VERIFY_DETERMINISM_REPORT)"',
    "$(MAKE) verify-determinism-check",
]

FORBIDDEN_HASH_SURFACE_TERMS = [
    "elapsed_nanos",
    "total_elapsed_nanos",
    "Instant",
    "SystemTime",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden {term}" for term in terms if term in text]


def function_slice(text: str, name: str) -> str:
    marker = f"fn {name}"
    start = text.find(marker)
    if start < 0:
        return ""
    candidates = [
        index
        for index in (
            text.find("\nfn ", start + 1),
            text.find("\n#[cfg(test)]", start + 1),
        )
        if index > start
    ]
    end = min(candidates) if candidates else len(text)
    return text[start:end]


def validate(root: Path) -> dict[str, Any]:
    canonical = read_text(root / "crates/cortex-engine/src/canonical.rs")
    determinism_tests = read_text(root / "crates/cortex-engine/tests/determinism.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    verification_report_surface = function_slice(canonical, "verification_report_value")
    numeric_conflict_surface = function_slice(canonical, "numeric_conflict_value")
    hashed_surface = "\n".join([verification_report_surface, numeric_conflict_surface])

    failures: list[str] = []
    failures.extend(missing_terms("canonical.rs", canonical, REQUIRED_CANONICAL_TERMS))
    failures.extend(
        missing_terms(
            "determinism/canonical tests",
            canonical + "\n" + determinism_tests,
            REQUIRED_TEST_TERMS,
        )
    )
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    if not verification_report_surface:
        failures.append("canonical.rs: missing verification_report_value body")
    if not numeric_conflict_surface:
        failures.append("canonical.rs: missing numeric_conflict_value body")
    failures.extend(
        forbidden_terms(
            "canonical verification hash surface",
            hashed_surface,
            FORBIDDEN_HASH_SURFACE_TERMS,
        )
    )

    return {
        "schema_version": "cortexdb.verify_determinism.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "canonical_terms": REQUIRED_CANONICAL_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_hash_surface_terms": FORBIDDEN_HASH_SURFACE_TERMS,
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
    print(f"verify determinism check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
