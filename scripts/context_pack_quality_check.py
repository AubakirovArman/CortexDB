#!/usr/bin/env python3
"""Validate ContextPack quality fixture metrics."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535


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
    deterministic_order_cases = 0

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

        required_evidence += int_field(case, "required_evidence")
        covered_evidence += int_field(case, "covered_evidence")
        raw_tokens += int_field(case, "raw_tokens")
        pack_tokens += int_field(case, "pack_tokens")
        redundant_candidates += int_field(case, "redundant_candidates")
        suppressed_redundant += int_field(case, "suppressed_redundant")
        if bool_field(case, "deterministic_order"):
            deterministic_order_cases += 1
        if bool_field(case, "citations_required"):
            required_citation_cells += int_field(case, "pack_cells")
            cited_cells += int_field(case, "cited_cells")

        if int_field(case, "pack_tokens") > int_field(case, "raw_tokens"):
            failures.append(f"{case_id}: pack_tokens exceeds raw_tokens")
        if int_field(case, "covered_evidence") > int_field(case, "required_evidence"):
            failures.append(f"{case_id}: covered_evidence exceeds required_evidence")
        if int_field(case, "suppressed_redundant") > int_field(case, "redundant_candidates"):
            failures.append(f"{case_id}: suppressed_redundant exceeds redundant_candidates")

    metrics = {
        "evidence_coverage_q16": q16(covered_evidence, required_evidence),
        "token_reduction_q16": q16(raw_tokens - pack_tokens, raw_tokens),
        "citation_coverage_q16": q16(cited_cells, required_citation_cells),
        "redundancy_reduction_q16": q16(suppressed_redundant, redundant_candidates),
        "deterministic_order_q16": q16(deterministic_order_cases, len(cases)),
    }
    if metrics["evidence_coverage_q16"] < Q16_ONE:
        failures.append("evidence coverage is below 100 percent")
    if metrics["token_reduction_q16"] <= 0:
        failures.append("token reduction must be positive")
    if metrics["citation_coverage_q16"] < Q16_ONE:
        failures.append("citation coverage is below 100 percent")
    if metrics["redundancy_reduction_q16"] < Q16_ONE:
        failures.append("redundancy reduction is below 100 percent")
    if metrics["deterministic_order_q16"] < Q16_ONE:
        failures.append("deterministic order coverage is below 100 percent")

    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "case_count": len(cases),
        "raw_tokens": raw_tokens,
        "pack_tokens": pack_tokens,
        **metrics,
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
