#!/usr/bin/env python3
"""Validate ContextPack quality fixture metrics."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535
MIN_BETA_CASES = 25
MIN_BETA_DOMAINS = 3
MIN_BETA_COVERAGE_Q16 = 60_000


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


def int_field(case: dict[str, Any], field: str) -> int:
    value = case.get(field)
    if not isinstance(value, int):
        raise ValueError(f"{case.get('case_id', '<unknown>')}:{field}: expected integer")
    return value


def bool_field(case: dict[str, Any], field: str) -> bool:
    value = case.get(field)
    if not isinstance(value, bool):
        raise ValueError(f"{case.get('case_id', '<unknown>')}:{field}: expected boolean")
    return value


def str_field(case: dict[str, Any], field: str) -> str:
    value = case.get(field)
    if not isinstance(value, str) or not value:
        raise ValueError(f"{case.get('case_id', '<unknown>')}:{field}: expected non-empty string")
    return value


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def validate_cases(cases: list[dict[str, Any]]) -> dict[str, Any]:
    if not cases:
        raise ValueError("expected at least one ContextPack quality case")

    failures: list[str] = []
    case_ids: set[str] = set()
    required_evidence = covered_evidence = 0
    raw_tokens = pack_tokens = 0
    required_citation_cells = cited_cells = 0
    redundant_candidates = suppressed_redundant = 0
    expected_anomalies = reported_anomalies = 0
    deterministic_order_cases = 0
    classic_rag_chunks = classic_rag_tokens = 0
    classic_rag_duplicates = classic_rag_anomalies = 0
    classic_rag_cited_chunks = 0
    domains: set[str] = set()
    per_domain: dict[str, dict[str, int]] = {}

    for case in cases:
        case_id = case.get("case_id")
        if not isinstance(case_id, str) or not case_id:
            failures.append("case_id must be a non-empty string")
            continue
        if case_id in case_ids:
            failures.append(f"duplicate case_id: {case_id}")
        case_ids.add(case_id)
        if not isinstance(case.get("scenario"), str) or not case["scenario"]:
            failures.append(f"{case_id}:scenario must be a non-empty string")
        domain = str_field(case, "domain")
        domains.add(domain)
        domain_totals = per_domain.setdefault(
            domain,
            {
                "case_count": 0,
                "required_evidence": 0,
                "covered_evidence": 0,
                "raw_tokens": 0,
                "pack_tokens": 0,
                "classic_rag_tokens": 0,
                "classic_rag_chunks": 0,
                "pack_cells": 0,
                "classic_rag_duplicate_chunks": 0,
                "required_citation_cells": 0,
                "cited_cells": 0,
                "redundant_candidates": 0,
                "suppressed_redundant": 0,
                "expected_anomalies": 0,
                "reported_anomalies": 0,
                "deterministic_order_cases": 0,
            },
        )
        domain_totals["case_count"] += 1

        required_evidence += int_field(case, "required_evidence")
        covered_evidence += int_field(case, "covered_evidence")
        raw_tokens += int_field(case, "raw_tokens")
        pack_tokens += int_field(case, "pack_tokens")
        redundant_candidates += int_field(case, "redundant_candidates")
        suppressed_redundant += int_field(case, "suppressed_redundant")
        expected_anomalies += int_field(case, "expected_anomalies")
        reported_anomalies += int_field(case, "reported_anomalies")
        classic_rag_chunks += int_field(case, "classic_rag_chunks")
        classic_rag_tokens += int_field(case, "classic_rag_tokens")
        classic_rag_duplicates += int_field(case, "classic_rag_duplicate_chunks")
        classic_rag_cited_chunks += int_field(case, "classic_rag_cited_chunks")
        classic_rag_anomalies += int_field(case, "classic_rag_anomalies")
        if bool_field(case, "deterministic_order"):
            deterministic_order_cases += 1
        if bool_field(case, "citations_required"):
            required_citation_cells += int_field(case, "pack_cells")
            cited_cells += int_field(case, "cited_cells")
            domain_totals["required_citation_cells"] += int_field(case, "pack_cells")
            domain_totals["cited_cells"] += int_field(case, "cited_cells")

        domain_totals["required_evidence"] += int_field(case, "required_evidence")
        domain_totals["covered_evidence"] += int_field(case, "covered_evidence")
        domain_totals["raw_tokens"] += int_field(case, "raw_tokens")
        domain_totals["pack_tokens"] += int_field(case, "pack_tokens")
        domain_totals["classic_rag_tokens"] += int_field(case, "classic_rag_tokens")
        domain_totals["classic_rag_chunks"] += int_field(case, "classic_rag_chunks")
        domain_totals["pack_cells"] += int_field(case, "pack_cells")
        domain_totals["classic_rag_duplicate_chunks"] += int_field(case, "classic_rag_duplicate_chunks")
        domain_totals["redundant_candidates"] += int_field(case, "redundant_candidates")
        domain_totals["suppressed_redundant"] += int_field(case, "suppressed_redundant")
        domain_totals["expected_anomalies"] += int_field(case, "expected_anomalies")
        domain_totals["reported_anomalies"] += int_field(case, "reported_anomalies")
        if bool_field(case, "deterministic_order"):
            domain_totals["deterministic_order_cases"] += 1

        if int_field(case, "pack_tokens") > int_field(case, "raw_tokens"):
            failures.append(f"{case_id}: pack_tokens exceeds raw_tokens")
        if int_field(case, "covered_evidence") > int_field(case, "required_evidence"):
            failures.append(f"{case_id}: covered_evidence exceeds required_evidence")
        if int_field(case, "suppressed_redundant") > int_field(case, "redundant_candidates"):
            failures.append(f"{case_id}: suppressed_redundant exceeds redundant_candidates")
        if int_field(case, "reported_anomalies") > int_field(case, "expected_anomalies"):
            failures.append(f"{case_id}: reported_anomalies exceeds expected_anomalies")
        if int_field(case, "pack_cells") > int_field(case, "classic_rag_chunks"):
            failures.append(f"{case_id}: pack_cells exceeds classic_rag_chunks")
        if int_field(case, "pack_tokens") >= int_field(case, "classic_rag_tokens"):
            failures.append(f"{case_id}: pack_tokens must be lower than classic_rag_tokens")

    metrics = {
        "domains": sorted(domains),
        "domain_count": len(domains),
        "classic_rag_chunks": classic_rag_chunks,
        "classic_rag_tokens": classic_rag_tokens,
        "classic_rag_duplicate_chunks": classic_rag_duplicates,
        "classic_rag_cited_chunks": classic_rag_cited_chunks,
        "classic_rag_anomalies": classic_rag_anomalies,
        "evidence_coverage_q16": q16(covered_evidence, required_evidence),
        "token_reduction_q16": q16(raw_tokens - pack_tokens, raw_tokens),
        "context_pack_token_savings_vs_classic_q16": q16(
            classic_rag_tokens - pack_tokens,
            classic_rag_tokens,
        ),
        "context_pack_cell_reduction_vs_classic_q16": q16(
            classic_rag_chunks - sum(int_field(case, "pack_cells") for case in cases),
            classic_rag_chunks,
        ),
        "classic_rag_duplicate_rate_q16": q16(classic_rag_duplicates, classic_rag_chunks),
        "citation_coverage_q16": q16(cited_cells, required_citation_cells),
        "redundancy_reduction_q16": q16(suppressed_redundant, redundant_candidates),
        "anomaly_coverage_q16": q16(reported_anomalies, expected_anomalies),
        "deterministic_order_q16": q16(deterministic_order_cases, len(cases)),
    }
    domain_metrics = {
        domain: {
            **totals,
            "evidence_coverage_q16": q16(totals["covered_evidence"], totals["required_evidence"]),
            "token_reduction_q16": q16(
                totals["raw_tokens"] - totals["pack_tokens"],
                totals["raw_tokens"],
            ),
            "context_pack_token_savings_vs_classic_q16": q16(
                totals["classic_rag_tokens"] - totals["pack_tokens"],
                totals["classic_rag_tokens"],
            ),
            "context_pack_cell_reduction_vs_classic_q16": q16(
                totals["classic_rag_chunks"] - totals["pack_cells"],
                totals["classic_rag_chunks"],
            ),
            "classic_rag_duplicate_rate_q16": q16(
                totals["classic_rag_duplicate_chunks"],
                totals["classic_rag_chunks"],
            ),
            "citation_coverage_q16": q16(
                totals["cited_cells"],
                totals["required_citation_cells"],
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
        for domain, totals in sorted(per_domain.items())
    }
    if len(cases) < MIN_BETA_CASES:
        failures.append(f"expected at least {MIN_BETA_CASES} ContextPack quality cases")
    if metrics["domain_count"] < MIN_BETA_DOMAINS:
        failures.append(f"expected at least {MIN_BETA_DOMAINS} ContextPack quality domains")
    if metrics["evidence_coverage_q16"] < MIN_BETA_COVERAGE_Q16:
        failures.append("evidence coverage is below beta threshold")
    if metrics["token_reduction_q16"] <= 0:
        failures.append("token reduction must be positive")
    if metrics["citation_coverage_q16"] < MIN_BETA_COVERAGE_Q16:
        failures.append("citation coverage is below beta threshold")
    if metrics["redundancy_reduction_q16"] < Q16_ONE:
        failures.append("redundancy reduction is below 100 percent")
    if metrics["anomaly_coverage_q16"] < Q16_ONE:
        failures.append("anomaly coverage is below 100 percent")
    if metrics["deterministic_order_q16"] < Q16_ONE:
        failures.append("deterministic order coverage is below 100 percent")
    if metrics["context_pack_token_savings_vs_classic_q16"] <= 0:
        failures.append("ContextPack does not save tokens versus classic RAG")
    if metrics["context_pack_cell_reduction_vs_classic_q16"] <= 0:
        failures.append("ContextPack does not reduce selected cells versus classic RAG")
    if metrics["classic_rag_duplicate_rate_q16"] <= 0:
        failures.append("classic RAG baseline must include duplicate chunk pressure")

    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "case_count": len(cases),
        "raw_tokens": raw_tokens,
        "pack_tokens": pack_tokens,
        **metrics,
        "per_domain_metrics": domain_metrics,
    }


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
        print(f"context pack quality check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"context pack quality check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
