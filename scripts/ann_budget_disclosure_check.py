#!/usr/bin/env python3
"""Validate the ANN budget disclosure gate for ContextPack."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_CONTEXT_TERMS = [
    "RetrievalIncomplete",
    'Self::RetrievalIncomplete => "retrieval_incomplete"',
]

REQUIRED_PLUMBING_TERMS = [
    "context_pack_from_search_outcome_with_options",
    "retrieval_incomplete_anomalies",
    "AnnFallbackReason::VisitBudgetExceeded",
    "AnnSloViolation::VisitBudgetExceeded",
    "ContextPackAnomalyCode::RetrievalIncomplete",
    "ANN visit budget was exhausted",
]

REQUIRED_TEST_TERMS = [
    "ann_visit_budget_exhaustion_is_disclosed_in_context_pack_exports",
    "complete_ann_search_does_not_report_retrieval_incomplete",
    "max_visited_candidates: Some(0)",
    "ContextPackExportFormat::Json",
    "ContextPackExportFormat::Prompt",
    "ContextPackExportFormat::Markdown",
]

REQUIRED_CONTRACT_TERMS = {
    "docs/schemas/context_pack.v1.json": ['"retrieval_incomplete"'],
    "docs/openapi.yaml": ["- retrieval_incomplete"],
    "sdk/python/_cortexdb_client/generated/openapi_types.py": ["retrieval_incomplete"],
    "sdk/typescript/cortexdb-client/generated/openapi-types.ts": ["retrieval_incomplete"],
}

REQUIRED_MAKE_TERMS = [
    "ANN_BUDGET_DISCLOSURE_REPORT ?= target/context-pack-quality/ann-budget-disclosure-report.json",
    "ann-budget-disclosure-check:",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    context_mod = read_text(root / "crates/cortex-engine/src/context/mod.rs")
    search_context = read_text(root / "crates/cortex-engine/src/search/database/context.rs")
    tests = read_text(root / "crates/cortex-engine/tests/ann_budget_disclosure.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures: list[str] = []
    failures.extend(missing_terms("context/mod.rs", context_mod, REQUIRED_CONTEXT_TERMS))
    failures.extend(
        missing_terms("search/database/context.rs", search_context, REQUIRED_PLUMBING_TERMS)
    )
    failures.extend(missing_terms("ann_budget_disclosure.rs", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    for relative, terms in REQUIRED_CONTRACT_TERMS.items():
        failures.extend(missing_terms(relative, read_text(root / relative), terms))

    return {
        "schema_version": "cortexdb.ann_budget_disclosure.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "context_terms": REQUIRED_CONTEXT_TERMS,
            "plumbing_terms": REQUIRED_PLUMBING_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "contract_terms": REQUIRED_CONTRACT_TERMS,
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
    print(f"ANN budget disclosure check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
