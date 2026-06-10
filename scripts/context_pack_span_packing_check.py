#!/usr/bin/env python3
"""Validate ContextPack span-level packing evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


Q16_ONE = 65_535


REQUIRED_TERMS = {
    "crates/cortex-engine/src/context/span.rs": [
        "select_relevant_span",
        "span_level_packing",
        "context_pack_span=true",
        "select_budgeted_window",
        "ContextSpanProvenance",
    ],
    "crates/cortex-engine/tests/context_pack_span_packing.rs": [
        "span_level_packing_beats_prefix_truncation_under_same_budget",
        "span_level_packing_preserves_citation_metadata",
        "span_packed_cells_export_structured_provenance",
    ],
    "docs/CONTEXT_PACK.md": [
        "span_level_packing",
        "context_pack_span=true",
        "source_byte_start",
        "context-pack-span-packing-check",
    ],
    "docs/CONTEXT_PACK_TECHNOLOGY.md": [
        "span_level_packing",
        "query-relevant body span",
        "structured provenance",
    ],
    "docs/CONTEXT_PACK_QUALITY_EVIDENCE.md": [
        "Span-Level Packing Evidence",
        "context_pack_span_packing",
        "structured span provenance",
    ],
}


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def load_cases(path: Path) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise SystemExit(f"{path}:{line_number}: expected JSON object")
            cases.append(value)
    if not cases:
        raise SystemExit(f"{path}: expected at least one case")
    return cases


def require_int(case: dict[str, Any], field: str) -> int:
    value = case.get(field)
    if not isinstance(value, int):
        raise SystemExit(f"{case.get('case_id', '<unknown>')}:{field}: expected integer")
    return value


def require_bool(case: dict[str, Any], field: str) -> bool:
    value = case.get(field)
    if not isinstance(value, bool):
        raise SystemExit(f"{case.get('case_id', '<unknown>')}:{field}: expected boolean")
    return value


def validate_fixture(cases: list[dict[str, Any]]) -> dict[str, Any]:
    required = prefix_covered = span_covered = 0
    prefix_tokens = span_tokens = 0
    failures: list[str] = []
    seen: set[str] = set()

    for case in cases:
        case_id = case.get("case_id")
        if not isinstance(case_id, str) or not case_id:
            failures.append("case_id must be non-empty")
            continue
        if case_id in seen:
            failures.append(f"{case_id}: duplicate case_id")
        seen.add(case_id)

        budget = require_int(case, "budget_tokens")
        case_required = require_int(case, "required_evidence")
        case_prefix = require_int(case, "prefix_covered_evidence")
        case_span = require_int(case, "span_covered_evidence")
        case_prefix_tokens = require_int(case, "prefix_tokens")
        case_span_tokens = require_int(case, "span_tokens")
        span_marker = require_bool(case, "span_marker")
        citations_preserved = require_bool(case, "citations_preserved")
        deterministic_order = require_bool(case, "deterministic_order")

        required += case_required
        prefix_covered += case_prefix
        span_covered += case_span
        prefix_tokens += case_prefix_tokens
        span_tokens += case_span_tokens

        if case_span <= case_prefix:
            failures.append(f"{case_id}: span coverage must exceed prefix coverage")
        if case_span > case_required:
            failures.append(f"{case_id}: span coverage exceeds required evidence")
        if case_span_tokens > budget:
            failures.append(f"{case_id}: span tokens exceed budget")
        if case_span_tokens > case_prefix_tokens:
            failures.append(f"{case_id}: span tokens exceed prefix tokens")
        if not span_marker:
            failures.append(f"{case_id}: span marker missing")
        if not citations_preserved:
            failures.append(f"{case_id}: citations not preserved")
        if not deterministic_order:
            failures.append(f"{case_id}: deterministic order not proven")

    if failures:
        raise SystemExit("\n".join(failures))

    return {
        "case_count": len(cases),
        "required_evidence": required,
        "prefix_covered_evidence": prefix_covered,
        "span_covered_evidence": span_covered,
        "prefix_coverage_q16": q16(prefix_covered, required),
        "span_coverage_q16": q16(span_covered, required),
        "coverage_lift_q16": q16(span_covered - prefix_covered, required),
        "prefix_tokens": prefix_tokens,
        "span_tokens": span_tokens,
        "span_token_savings_vs_prefix_q16": q16(prefix_tokens - span_tokens, prefix_tokens),
    }


def validate_terms(root: Path) -> list[dict[str, object]]:
    results: list[dict[str, object]] = []
    for relative, terms in REQUIRED_TERMS.items():
        path = root / relative
        if not path.exists():
            raise SystemExit(f"missing required file: {relative}")
        text = path.read_text(encoding="utf-8")
        missing = [term for term in terms if term not in text]
        if missing:
            raise SystemExit(f"{relative}: missing terms: {', '.join(missing)}")
        results.append({"path": relative, "checked_terms": terms, "status": "ok"})
    return results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--fixture", default="examples/eval/context_pack_span_packing.jsonl")
    parser.add_argument("--report", required=True)
    args = parser.parse_args()

    root = Path(args.root)
    fixture = root / args.fixture
    metrics = validate_fixture(load_cases(fixture))
    checks = validate_terms(root)
    report = Path(args.report)
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(
        json.dumps(
            {
                "gate": "context_pack_span_packing",
                "status": "passed",
                "fixture": str(fixture),
                "metrics": metrics,
                "checks": checks,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"context pack span packing check passed: {report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
