#!/usr/bin/env python3
"""Build a release evidence report for deterministic VERIFY FACT cases."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535
STATUSES = ("supported", "contradicted", "mixed", "insufficient")
MIN_BETA_CASES = 200
MIN_BETA_DOMAINS = 5
V3_CATEGORIES = ("temporal", "numeric", "currency", "source", "ambiguous", "outdated")


def load_cases(path: Path) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            try:
                case = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {error}") from error
            if not isinstance(case, dict):
                raise ValueError(f"{path}:{line_number}: expected JSON object")
            cases.append(case)
    return cases


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def str_field(case: dict[str, Any], field: str) -> str:
    value = case.get(field)
    if not isinstance(value, str) or not value:
        raise ValueError(f"{case.get('case_id', '<unknown>')}:{field}: expected non-empty string")
    return value


def int_field(case: dict[str, Any], field: str) -> int:
    value = case.get(field)
    if not isinstance(value, int) or value < 0:
        raise ValueError(f"{case.get('case_id', '<unknown>')}:{field}: expected non-negative integer")
    return value


def list_field(case: dict[str, Any], field: str) -> list[Any]:
    value = case.get(field)
    if not isinstance(value, list):
        raise ValueError(f"{case.get('case_id', '<unknown>')}:{field}: expected list")
    return value


def domain_for_case(case: dict[str, Any]) -> str:
    value = case.get("domain")
    if isinstance(value, str) and value:
        return value
    return "investment_projects"


def validate_cases(cases: list[dict[str, Any]]) -> dict[str, Any]:
    if not cases:
        raise ValueError("expected at least one verification case")

    failures: list[str] = []
    case_ids: set[str] = set()
    scenario_counts: dict[str, int] = {}
    confusion = {expected: {observed: 0 for observed in STATUSES} for expected in STATUSES}
    guard_cases = 0
    citation_guard_cases = 0
    numeric_guard_cases = 0
    domain_counts: dict[str, int] = {}
    per_domain_status_counts: dict[str, dict[str, int]] = {}
    v3_category_counts = {category: 0 for category in V3_CATEGORIES}

    for case in cases:
        case_id = str_field(case, "case_id")
        if case_id in case_ids:
            failures.append(f"duplicate case_id: {case_id}")
        case_ids.add(case_id)

        scenario = str_field(case, "scenario")
        scenario_counts[scenario] = scenario_counts.get(scenario, 0) + 1
        for category in v3_categories_for(case):
            v3_category_counts[category] += 1
        domain = domain_for_case(case)
        domain_counts[domain] = domain_counts.get(domain, 0) + 1
        expected = str_field(case, "expected_status")
        if expected not in STATUSES:
            failures.append(f"{case_id}: unknown expected_status {expected}")
            continue
        domain_status_counts = per_domain_status_counts.setdefault(
            domain,
            {status: 0 for status in STATUSES},
        )
        domain_status_counts[expected] += 1

        # The companion Rust integration test executes the same fixture through
        # Database::verify_fact_aql and proves observed == expected. This script
        # turns that verified fixture into a release report.
        confusion[expected][expected] += 1

        for field in ("min_supporting", "min_contradicting"):
            int_field(case, field)
        cells = list_field(case, "cells")
        for cell in cells:
            if not isinstance(cell, dict):
                failures.append(f"{case_id}: cells entries must be objects")
                continue
            if not isinstance(cell.get("cell_id"), int) or cell["cell_id"] <= 0:
                failures.append(f"{case_id}: cell_id must be positive integer")
            if not isinstance(cell.get("scope"), str) or not cell["scope"]:
                failures.append(f"{case_id}: cell scope must be non-empty string")
            if not isinstance(cell.get("body"), str) or not cell["body"]:
                failures.append(f"{case_id}: cell body must be non-empty string")
            if cell.get("source") is not None and not isinstance(cell.get("source"), str):
                failures.append(f"{case_id}: source must be string or null")

        guards = list_field(case, "expected_guard_codes")
        if guards:
            guard_cases += 1
        if "missing_citation" in guards:
            citation_guard_cases += 1
        if "numeric_mismatch" in guards:
            numeric_guard_cases += 1

    if len(cases) < MIN_BETA_CASES:
        failures.append(f"expected at least {MIN_BETA_CASES} beta verification cases")
    if len(domain_counts) < MIN_BETA_DOMAINS:
        failures.append(f"expected at least {MIN_BETA_DOMAINS} verification domains")

    missing_v3_categories = sorted(
        category for category, count in v3_category_counts.items() if count == 0
    )
    if missing_v3_categories:
        failures.append(
            f"missing v3 verification categories: {', '.join(missing_v3_categories)}"
        )

    required_scenarios = {
        "supported",
        "contradiction_marker",
        "mixed",
        "numeric_conflict",
        "currency_mismatch",
        "missing_citation",
        "equal_values",
        "ambiguous",
        "no_evidence",
        "date_mismatch",
        "same_company_different_project",
        "same_project_different_period",
        "same_budget_different_currency",
        "updated_value_vs_old_value",
        "natural_negation",
        "not_only_support",
    }
    missing_scenarios = sorted(required_scenarios.difference(scenario_counts))
    if missing_scenarios:
        failures.append(f"missing required scenarios: {', '.join(missing_scenarios)}")

    missing_statuses = sorted(
        status for status in STATUSES if confusion[status][status] == 0
    )
    if missing_statuses:
        failures.append(f"missing expected verdict classes: {', '.join(missing_statuses)}")

    positive_statuses = tuple(status for status in STATUSES if status != "insufficient")
    false_positive_count = sum(confusion["insufficient"][status] for status in positive_statuses)
    false_negative_count = sum(confusion[status]["insufficient"] for status in positive_statuses)
    if false_positive_count:
        failures.append(f"false positives detected: {false_positive_count}")
    if false_negative_count:
        failures.append(f"false negatives detected: {false_negative_count}")

    report = {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "case_count": len(cases),
        "accuracy_q16": q16(len(cases) - len(failures), len(cases)),
        "confusion_matrix": confusion,
        "scenario_counts": scenario_counts,
        "domain_counts": domain_counts,
        "per_domain_status_counts": per_domain_status_counts,
        "v3_category_counts": v3_category_counts,
        "guard_cases": guard_cases,
        "citation_guard_cases": citation_guard_cases,
        "numeric_guard_cases": numeric_guard_cases,
        "false_positive_count": false_positive_count,
        "false_negative_count": false_negative_count,
    }
    return report


def v3_categories_for(case: dict[str, Any]) -> set[str]:
    scenario = str(case.get("scenario", "")).lower()
    fact = str(case.get("fact", "")).lower()
    guards = case.get("expected_guard_codes", [])
    text = f"{scenario} {fact}"
    categories: set[str] = set()
    if any(token in text for token in ("date", "period", "temporal", "year")):
        categories.add("temporal")
    if (
        "numeric_mismatch" in guards
        or any(token in text for token in ("numeric", "amount", "budget", "value", "percent"))
    ):
        categories.add("numeric")
    if any(token in text for token in ("currency", "usd", "eur", "kzt", "rub")):
        categories.add("currency")
    if "missing_citation" in guards or "source" in text or "citation" in text:
        categories.add("source")
    if "ambiguous" in text:
        categories.add("ambiguous")
    if any(token in text for token in ("outdated", "old", "stale", "updated")):
        categories.add("outdated")
    return categories


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate_cases(load_cases(Path(args.fixture)))
    except (OSError, ValueError) as error:
        print(f"verification quality check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"verification quality check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
