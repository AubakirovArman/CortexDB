#!/usr/bin/env python3
"""Validate the DV7 VERIFY conflict recall benchmark report."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "cortexdb.verify_conflict_recall.report.v1"
MIN_CASES = 150
MIN_RECALL_Q16 = 58_981
MAX_FALSE_CONFLICT_RATE_Q16 = 3_276
REQUIRED_CLASSES = (
    "magnitude",
    "unit",
    "currency",
    "temporal",
    "citation",
    "format",
    "must_not_conflict",
)

REQUIRED_TERMS = {
    "crates/cortex-engine/tests/verification_conflict_recall.rs": [
        "verification_conflict_recall_benchmark_meets_thresholds",
        "report::run_benchmark()",
    ],
    "crates/cortex-engine/tests/verification_conflict_recall/cases.rs": [
        "CONFLICT_REPEATS: usize = 25",
        "CONTROL_REPEATS: usize = 30",
        "VerificationNumericConflictKind::Temporal",
        "VerificationNumericConflictKind::Citation",
        "must_not_conflict",
        "60 min",
        "2 h",
        "$1.2M",
    ],
    "crates/cortex-engine/tests/verification_conflict_recall/report.rs": [
        "MIN_RECALL_Q16: u32 = 58_981",
        "MAX_FALSE_CONFLICT_RATE_Q16: u32 = 3_276",
        "cortexdb.verify_conflict_recall.report.v1",
    ],
    "mk/core-retrieval-context.mk": [
        "verify-conflict-recall-check:",
        'CORTEXDB_VERIFY_CONFLICT_RECALL_REPORT="$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)"',
        'python3 scripts/verify_conflict_recall_check.py --root "." --report "$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "VERIFY_CONFLICT_RECALL_REPORT ?= target/verification-quality/conflict-recall-report.json",
    ],
    "mk/phony.mk": [
        "verify-conflict-recall-check",
    ],
}


def read_text(root: Path, relative: str) -> str:
    path = root / relative
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def load_report(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise ValueError(
            f"missing measured report {path}; run make verify-conflict-recall-check first"
        )
    with path.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError("measured report must be a JSON object")
    return data


def int_field(report: dict[str, Any], field: str) -> int:
    value = report.get(field)
    if not isinstance(value, int):
        raise ValueError(f"{field}: expected integer")
    return value


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if report.get("schema_version") != SCHEMA_VERSION:
        failures.append("schema_version mismatch")
    if report.get("status") != "passed":
        failures.append("measured report status is not passed")
    if report.get("failures"):
        failures.append("measured report contains case failures")

    case_count = int_field(report, "case_count")
    conflict_case_count = int_field(report, "conflict_case_count")
    no_conflict_case_count = int_field(report, "no_conflict_case_count")
    recall_q16 = int_field(report, "recall_q16")
    false_conflict_rate_q16 = int_field(report, "false_conflict_rate_q16")
    precision_q16 = int_field(report, "precision_q16")
    false_negative_count = int_field(report, "false_negative_count")
    false_conflict_count = int_field(report, "false_conflict_count")

    if case_count < MIN_CASES:
        failures.append(f"case_count={case_count} below {MIN_CASES}")
    if conflict_case_count <= 0:
        failures.append("conflict_case_count must be positive")
    if no_conflict_case_count <= 0:
        failures.append("no_conflict_case_count must be positive")
    if recall_q16 < MIN_RECALL_Q16:
        failures.append(f"recall_q16={recall_q16} below {MIN_RECALL_Q16}")
    if false_conflict_rate_q16 > MAX_FALSE_CONFLICT_RATE_Q16:
        failures.append(
            f"false_conflict_rate_q16={false_conflict_rate_q16} above {MAX_FALSE_CONFLICT_RATE_Q16}"
        )
    if precision_q16 <= 0:
        failures.append("precision_q16 must be positive")
    if false_negative_count != 0:
        failures.append(f"false_negative_count={false_negative_count}, expected 0")
    if false_conflict_count != 0:
        failures.append(f"false_conflict_count={false_conflict_count}, expected 0")

    class_counts = report.get("class_counts")
    if not isinstance(class_counts, dict):
        failures.append("class_counts: expected object")
        return failures
    for class_name in REQUIRED_CLASSES:
        stats = class_counts.get(class_name)
        if not isinstance(stats, dict):
            failures.append(f"missing class {class_name}")
            continue
        if not isinstance(stats.get("case_count"), int) or stats["case_count"] <= 0:
            failures.append(f"class {class_name} has no cases")
    return failures


def validate_wiring(root: Path) -> list[str]:
    failures: list[str] = []
    for relative, terms in REQUIRED_TERMS.items():
        try:
            text = read_text(root, relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{relative}: missing marker {term}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="measured report path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    report_path = Path(args.report)
    try:
        report = load_report(report_path)
        failures = validate_report(report) + validate_wiring(root)
    except (OSError, ValueError) as error:
        failures = [str(error)]

    if failures:
        for failure in failures:
            print(failure)
        return 1
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"verify conflict recall check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
