#!/usr/bin/env python3
"""Validate VERIFY FACT documentation claims against the DV7 recall report."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "cortexdb.verify_docs_claims.report.v1"
RECALL_SCHEMA_VERSION = "cortexdb.verify_conflict_recall.report.v1"

REQUIRED_MAKE_TERMS = {
    "mk/core-retrieval-context.mk": [
        "verify-docs-claims-check:",
        "docs-claims-check:",
        "$(MAKE) verify-docs-claims-check",
        'python3 scripts/verify_docs_claims_check.py --root "." --recall-report "$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)" --report "$(CURDIR)/$(VERIFY_DOCS_CLAIMS_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "VERIFY_DOCS_CLAIMS_REPORT ?= target/verification-quality/docs-claims-report.json",
    ],
    "mk/phony.mk": [
        "verify-docs-claims-check",
        "docs-claims-check",
    ],
}


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise ValueError(f"missing report: {path}")
    with path.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: expected JSON object")
    return data


def int_field(report: dict[str, Any], field: str) -> int:
    value = report.get(field)
    if not isinstance(value, int):
        raise ValueError(f"{field}: expected integer")
    return value


def str_field(report: dict[str, Any], field: str) -> str:
    value = report.get(field)
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field}: expected non-empty string")
    return value


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def doc_requirements(recall: dict[str, Any]) -> list[str]:
    return [
        "## Measured Conflict Coverage",
        "report schema `cortexdb.verify_conflict_recall.report.v1`",
        f"Case count: {int_field(recall, 'case_count')}",
        f"Conflict cases: {int_field(recall, 'conflict_case_count')}",
        f"must-NOT-conflict controls: {int_field(recall, 'no_conflict_case_count')}",
        f"Conflict recall: {str_field(recall, 'recall_percent')} (`recall_q16={int_field(recall, 'recall_q16')}`",
        f"Precision: {str_field(recall, 'precision_percent')} (`precision_q16={int_field(recall, 'precision_q16')}`",
        (
            f"False-conflict rate: {str_field(recall, 'false_conflict_rate_percent')} "
            f"(`false_conflict_rate_q16={int_field(recall, 'false_conflict_rate_q16')}`"
        ),
        "magnitude/numeric",
        "unit-class time conversion",
        "currency-mismatch",
        "temporal same-date",
        "citation same-source",
        "format variants",
        "must-NOT-conflict controls",
        "no FX conversion",
        "make verify-conflict-recall-check",
        "make docs-claims-check",
    ]


def validate_docs(root: Path, recall: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    text = read_text(root / "docs/VERIFY_FACT.md")
    for marker in doc_requirements(recall):
        if not contains_marker(text, marker):
            failures.append(f"docs/VERIFY_FACT.md: missing marker {marker}")
    if "## Limitations (Alpha)" in text:
        failures.append("docs/VERIFY_FACT.md: stale Limitations (Alpha) section remains")
    return failures


def validate_recall_report(recall: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if recall.get("schema_version") != RECALL_SCHEMA_VERSION:
        failures.append("recall report schema mismatch")
    if recall.get("status") != "passed":
        failures.append("recall report did not pass")
    for field in (
        "case_count",
        "conflict_case_count",
        "no_conflict_case_count",
        "recall_q16",
        "precision_q16",
        "false_conflict_rate_q16",
    ):
        int_field(recall, field)
    for field in ("recall_percent", "precision_percent", "false_conflict_rate_percent"):
        str_field(recall, field)
    return failures


def validate_make_wiring(root: Path) -> list[str]:
    failures: list[str] = []
    for relative, terms in REQUIRED_MAKE_TERMS.items():
        try:
            text = read_text(root / relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{relative}: missing marker {term}")
    return failures


def write_report(path: Path, status: str, failures: list[str], recall: dict[str, Any]) -> None:
    output = {
        "schema_version": SCHEMA_VERSION,
        "status": status,
        "failures": failures,
        "source_recall_schema": recall.get("schema_version"),
        "source_metrics": {
            "case_count": recall.get("case_count"),
            "recall_q16": recall.get("recall_q16"),
            "precision_q16": recall.get("precision_q16"),
            "false_conflict_rate_q16": recall.get("false_conflict_rate_q16"),
        },
        "docs_checked": ["docs/VERIFY_FACT.md"],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--recall-report", required=True, help="DV7 measured recall report")
    parser.add_argument("--report", required=True, help="output docs-claims report")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    try:
        recall = load_json(Path(args.recall_report))
        failures = (
            validate_recall_report(recall)
            + validate_docs(root, recall)
            + validate_make_wiring(root)
        )
    except (OSError, ValueError) as error:
        recall = {}
        failures = [str(error)]

    status = "failed" if failures else "passed"
    write_report(Path(args.report), status, failures, recall)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print(f"verify docs claims check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
