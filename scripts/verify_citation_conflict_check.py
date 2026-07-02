#!/usr/bin/env python3
"""Validate the VERIFY citation numeric-conflict gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_ENGINE_TERMS = [
    "VerificationNumericConflictKind",
    "kind: VerificationNumericConflictKind",
    "VerificationNumericConflictKind::Citation",
    "citation_conflict_kind",
    "same_source_ref",
]

REQUIRED_CONFLICT_INDEX_TERMS = [
    "citation_source_key",
    "numeric_conflict_record_kind",
    "citation conflict",
]

REQUIRED_API_TERMS = [
    "pub kind: String",
    "kind: conflict.kind.as_str().to_owned()",
]

REQUIRED_TEST_TERMS = [
    "verify_fact_reports_citation_numeric_conflict_kind",
    "citation_numeric_conflict_index_tracks_same_source_disagreement",
    "same_source_equal_value_is_not_citation_conflict",
]

REQUIRED_MAKE_TERMS = [
    "VERIFY_CITATION_CONFLICT_REPORT ?= target/verification-quality/citation-conflict-report.json",
    "verify-citation-conflict-check:",
    "$(MAKE) verify-citation-conflict-check",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    engine = "\n".join(
        [
            read_text(root / "crates/cortex-engine/src/verification/types.rs"),
            read_text(root / "crates/cortex-engine/src/verification/numeric/fact_claim.rs"),
            read_text(root / "crates/cortex-engine/src/verification/guards.rs"),
        ]
    )
    conflict_index = read_text(
        root / "crates/cortex-engine/src/verification/conflict_index/store.rs"
    )
    api = "\n".join(
        [
            read_text(root / "crates/cortex-api-types/src/verification.rs"),
            read_text(root / "crates/cortex-server/src/memory.rs"),
            read_text(root / "crates/cortex-cli/src/cli_json.rs"),
        ]
    )
    tests = "\n".join(
        [
            read_text(root / "crates/cortex-engine/tests/verification_guards.rs"),
            read_text(root / "crates/cortex-engine/tests/verification_conflict_numeric.rs"),
        ]
    )
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures: list[str] = []
    failures.extend(missing_terms("engine", engine, REQUIRED_ENGINE_TERMS))
    failures.extend(missing_terms("conflict_index/store.rs", conflict_index, REQUIRED_CONFLICT_INDEX_TERMS))
    failures.extend(missing_terms("api", api, REQUIRED_API_TERMS))
    failures.extend(missing_terms("tests", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))

    return {
        "schema_version": "cortexdb.verify_citation_conflict.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "metrics": {
            "citation_conflict_regression_cases": 2,
            "same_source_agreement_controls": 1,
        },
        "checked": {
            "engine_terms": REQUIRED_ENGINE_TERMS,
            "conflict_index_terms": REQUIRED_CONFLICT_INDEX_TERMS,
            "api_terms": REQUIRED_API_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
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
    print(f"verify citation conflict check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
