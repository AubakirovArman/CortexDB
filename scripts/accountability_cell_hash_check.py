#!/usr/bin/env python3
"""Validate the accountability cell content hash gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_ACCOUNTABILITY_TERMS = [
    "ACCOUNTABILITY_CELL_BYTES_SCHEMA",
    "ACCOUNTABILITY_CELL_HASH_DOMAIN",
    "canonical_cell_bytes",
    "cell_content_hash",
    "retrieved_cell_content_hash",
    "blake3_256_domain",
    '"payload_hex"',
    '"descriptor"',
    '"content_hash"',
]

REQUIRED_TEST_TERMS = [
    "accountability_cell_content_hash_is_deterministic",
    "accountability_cell_content_hash_changes_on_one_payload_byte",
    "accountability_cell_content_hash_changes_on_descriptor_metadata_mutation",
    "accountability_cell_content_hash_is_not_payload_string_content_hash",
    "accountability_cell_content_hash_uses_domain_separated_blake3",
]

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_CELL_HASH_REPORT ?= target/accountability-cell-hash/report.json",
    "accountability-cell-hash-check:",
    "cargo test -p cortex-engine --test accountability_cell_hash --all-features",
    'python3 scripts/accountability_cell_hash_check.py --root "." --report "$(ACCOUNTABILITY_CELL_HASH_REPORT)"',
]

REQUIRED_PHONY_TERMS = [
    "accountability-cell-hash-check",
]

FORBIDDEN_ACCOUNTABILITY_TERMS = [
    "std::time",
    "Instant::now",
    "SystemTime::now",
    "DefaultHasher",
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
    accountability = read_text(root / "crates/cortex-engine/src/accountability.rs")
    tests = read_text(root / "crates/cortex-engine/tests/accountability_cell_hash.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")

    failures: list[str] = []
    failures.extend(
        missing_terms("accountability.rs", accountability, REQUIRED_ACCOUNTABILITY_TERMS)
    )
    failures.extend(missing_terms("accountability_cell_hash.rs", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", phony, REQUIRED_PHONY_TERMS))
    failures.extend(
        forbidden_terms("accountability.rs", accountability, FORBIDDEN_ACCOUNTABILITY_TERMS)
    )

    return {
        "schema_version": "cortexdb.accountability_cell_hash.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "accountability_terms": REQUIRED_ACCOUNTABILITY_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "phony_terms": REQUIRED_PHONY_TERMS,
            "forbidden_terms": FORBIDDEN_ACCOUNTABILITY_TERMS,
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
    print(f"accountability cell hash check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
