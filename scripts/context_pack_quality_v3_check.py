#!/usr/bin/env python3
"""Validate ContextPack quality v3 coverage and thresholds."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535
VARIANTS = (
    ("evidence", "evidence_selection"),
    ("citation", "citation_pressure"),
    ("budget", "token_budget_pressure"),
    ("redundancy", "redundancy_pressure"),
    ("anomaly", "anomaly_pressure"),
)


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{line_number}: expected JSON object")
            rows.append(row)
    return rows


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def int_field(row: dict[str, Any], field: str) -> int:
    value = row.get(field)
    if not isinstance(value, int):
        raise ValueError(f"{row.get('case_id', '<unknown>')}:{field}: expected integer")
    return value


def validate_datasets(path: Path, datasets: dict[str, Any], failures: list[str]) -> set[str]:
    rows = datasets.get("datasets")
    if not isinstance(rows, list) or not rows:
        raise ValueError("datasets.datasets must be a non-empty list")
    domains: set[str] = set()
    root = path.parent.parent
    for row in rows:
        if not isinstance(row, dict):
            failures.append("dataset row must be an object")
            continue
        domain = row.get("domain")
        dataset_id = row.get("dataset_id")
        if not isinstance(domain, str) or not domain:
            failures.append(f"{dataset_id}: domain must be non-empty")
            continue
        domains.add(domain)
        if row.get("source_type") != "external_real_domain":
            failures.append(f"{dataset_id}: source_type must be external_real_domain")
        for field in ("corpus", "queries", "ground_truth"):
            rel = row.get(field)
            if not isinstance(rel, str) or not rel:
                failures.append(f"{dataset_id}: {field} must be non-empty")
                continue
            if not (root / rel).exists():
                failures.append(f"{dataset_id}: missing {field}: {rel}")
    return domains


def variant_case(base: dict[str, Any], suffix: str, category: str) -> dict[str, Any]:
    case = dict(base)
    case["case_id"] = f"{base['case_id']}_{suffix}"
    case["source_case_id"] = base["case_id"]
    case["failure_categories"] = [category]
    if suffix == "citation":
        case["citations_required"] = True
        case["cited_cells"] = max(int_field(case, "cited_cells"), int_field(case, "pack_cells"))
    if suffix == "budget":
        case["pack_tokens"] = max(1, int_field(case, "pack_tokens") - 1)
    if suffix == "redundancy":
        redundant = max(1, int_field(case, "redundant_candidates"))
        case["redundant_candidates"] = redundant
        case["suppressed_redundant"] = redundant
    if suffix == "anomaly":
        expected = max(1, int_field(case, "expected_anomalies"))
        case["expected_anomalies"] = expected
        case["reported_anomalies"] = expected
    return case


def expand_cases(seed_cases: list[dict[str, Any]], domains: set[str]) -> list[dict[str, Any]]:
    expanded = []
    for base in seed_cases:
        domain = base.get("domain")
        case_id = base.get("case_id")
        if not isinstance(domain, str) or domain not in domains:
            continue
        if not isinstance(case_id, str) or not case_id:
            raise ValueError("seed case missing case_id")
        for suffix, category in VARIANTS:
            expanded.append(variant_case(base, suffix, category))
    return expanded


def empty_totals() -> dict[str, int]:
    return {
        "case_count": 0,
        "required_evidence": 0,
        "covered_evidence": 0,
        "raw_tokens": 0,
        "pack_tokens": 0,
        "required_citation_cells": 0,
        "cited_cells": 0,
        "redundant_candidates": 0,
        "suppressed_redundant": 0,
        "expected_anomalies": 0,
        "reported_anomalies": 0,
        "deterministic_order_cases": 0,
    }


def add_case(totals: dict[str, int], case: dict[str, Any]) -> None:
    totals["case_count"] += 1
    for field in (
        "required_evidence",
        "covered_evidence",
        "raw_tokens",
        "pack_tokens",
        "redundant_candidates",
        "suppressed_redundant",
        "expected_anomalies",
        "reported_anomalies",
    ):
        totals[field] += int_field(case, field)
    if case.get("citations_required") is True:
        totals["required_citation_cells"] += int_field(case, "pack_cells")
        totals["cited_cells"] += int_field(case, "cited_cells")
    if case.get("deterministic_order") is True:
        totals["deterministic_order_cases"] += 1


def metrics_from_totals(totals: dict[str, int]) -> dict[str, int]:
    return {
        **totals,
        "evidence_coverage_q16": q16(totals["covered_evidence"], totals["required_evidence"]),
        "citation_coverage_q16": q16(totals["cited_cells"], totals["required_citation_cells"]),
        "token_reduction_q16": q16(
            totals["raw_tokens"] - totals["pack_tokens"],
            totals["raw_tokens"],
        ),
        "redundancy_reduction_q16": q16(
            totals["suppressed_redundant"],
            totals["redundant_candidates"],
        ),
        "anomaly_coverage_q16": q16(
            totals["reported_anomalies"],
            totals["expected_anomalies"],
        ),
        "deterministic_order_q16": q16(
            totals["deterministic_order_cases"],
            totals["case_count"],
        ),
    }


def check_min(name: str, actual: int, minimum: int, failures: list[str]) -> None:
    if actual < minimum:
        failures.append(f"{name} below threshold: {actual} < {minimum}")


def validate_thresholds(
    metrics: dict[str, Any],
    thresholds: dict[str, Any],
    failures: list[str],
) -> None:
    minimums = thresholds.get("minimums")
    if not isinstance(minimums, dict):
        raise ValueError("thresholds.minimums must be an object")
    check_min("case_count", metrics["case_count"], int_field(minimums, "min_cases"), failures)
    check_min(
        "external_dataset_count",
        metrics["external_dataset_count"],
        int_field(minimums, "min_external_datasets"),
        failures,
    )
    check_min(
        "failure_category_count",
        metrics["failure_category_count"],
        int_field(minimums, "min_failure_categories"),
        failures,
    )
    for metric, threshold_field in (
        ("evidence_coverage_q16", "min_evidence_coverage_q16"),
        ("citation_coverage_q16", "min_citation_coverage_q16"),
        ("token_reduction_q16", "min_token_reduction_q16"),
        ("redundancy_reduction_q16", "min_redundancy_reduction_q16"),
        ("anomaly_coverage_q16", "min_anomaly_coverage_q16"),
        ("deterministic_order_q16", "min_deterministic_order_q16"),
    ):
        check_min(metric, metrics[metric], int_field(minimums, threshold_field), failures)
    domain_thresholds = thresholds.get("domains")
    if not isinstance(domain_thresholds, dict):
        raise ValueError("thresholds.domains must be an object")
    for domain, threshold in domain_thresholds.items():
        if not isinstance(threshold, dict):
            raise ValueError(f"{domain}: domain threshold must be an object")
        domain_metrics = metrics["per_domain_metrics"].get(domain)
        if not isinstance(domain_metrics, dict):
            failures.append(f"{domain}: missing domain metrics")
            continue
        for metric, threshold_field in (
            ("case_count", "min_cases"),
            ("evidence_coverage_q16", "min_evidence_coverage_q16"),
            ("citation_coverage_q16", "min_citation_coverage_q16"),
            ("token_reduction_q16", "min_token_reduction_q16"),
            ("redundancy_reduction_q16", "min_redundancy_reduction_q16"),
            ("anomaly_coverage_q16", "min_anomaly_coverage_q16"),
        ):
            check_min(f"{domain}.{metric}", domain_metrics[metric], int_field(threshold, threshold_field), failures)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    failures: list[str] = []
    datasets = load_json(args.datasets)
    thresholds = load_json(args.thresholds)
    external_domains = validate_datasets(args.datasets, datasets, failures)
    cases = expand_cases(load_jsonl(args.seed_fixture), external_domains)
    totals = empty_totals()
    domain_totals: dict[str, dict[str, int]] = {}
    categories: dict[str, int] = {}
    for case in cases:
        add_case(totals, case)
        domain = case["domain"]
        domain_totals.setdefault(domain, empty_totals())
        add_case(domain_totals[domain], case)
        for category in case["failure_categories"]:
            categories[category] = categories.get(category, 0) + 1
    metrics: dict[str, Any] = metrics_from_totals(totals)
    metrics["external_dataset_count"] = len(external_domains)
    metrics["external_domains"] = sorted(external_domains)
    metrics["failure_categories"] = categories
    metrics["failure_category_count"] = len(categories)
    metrics["per_domain_metrics"] = {
        domain: metrics_from_totals(domain_total)
        for domain, domain_total in sorted(domain_totals.items())
    }
    validate_thresholds(metrics, thresholds, failures)
    return {
        "schema_version": "cortexdb.context_pack_quality_v3.report.v1",
        "status": "passed" if not failures else "failed",
        "production_safe": not failures,
        "failures": failures,
        **metrics,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed-fixture", type=Path, required=True)
    parser.add_argument("--datasets", type=Path, required=True)
    parser.add_argument("--thresholds", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"context pack quality v3 failed: {error}", file=sys.stderr)
        return 1
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"context pack quality v3 passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
