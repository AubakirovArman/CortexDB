#!/usr/bin/env python3
"""Validate sparse-scope ANN exact-fallback recall wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-engine/src/search/ann.rs": [
        "mod sparse_scope_tests;",
    ],
    "crates/cortex-engine/src/search/ann/sparse_scope_tests.rs": [
        "sparse_allowed_set_routes_to_exact_before_hnsw_budget",
        "SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES",
        "AnnFallbackReason::SparseAllowedSet",
        "max_visited_candidates: Some(1)",
        "visited_candidates, 0",
    ],
    "crates/cortex-engine/src/search/ann/search.rs": [
        "SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES",
        "should_use_sparse_allowed_exact_fallback",
        "policy.max_visited_candidates.is_some()",
        "available.saturating_mul(4) <= graph_nodes",
        "AnnFallbackReason::SparseAllowedSet",
    ],
    "crates/cortex-engine/src/search/ann/types.rs": [
        "SparseAllowedSet",
        '"sparse_allowed_set"',
    ],
    "crates/cortex-engine/src/search/ann/report.rs": [
        "AnnFallbackReason::SparseAllowedSet => AnnSloViolation::SparseAllowedSet",
    ],
    "mk/core-retrieval-context.mk": [
        "ann-sparse-scope-recall-check:",
        "cargo test -p cortex-engine sparse_allowed_set_routes_to_exact_before_hnsw_budget --all-features",
        'python3 scripts/ann_sparse_scope_recall_check.py --root "." --report "$(ANN_SPARSE_SCOPE_RECALL_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "ANN_SPARSE_SCOPE_RECALL_REPORT ?= target/ann-sparse-scope-recall/report.json",
    ],
    "mk/phony.mk": [
        "ann-sparse-scope-recall-check",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "ann-sparse-scope-recall-check",
        "FC-3",
        "sparse_allowed_set",
        "SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES",
    ],
}

FORBIDDEN_TERMS = {
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "full per-scope ANN subgraph partitioning is done",
        "fallback-disabled sparse ANN is production-safe",
    ],
}


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, dict[str, list[str]]] = {}
    for rel, terms in REQUIRED_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["required"] = terms
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing ann sparse-scope marker: {term}")
    for rel, terms in FORBIDDEN_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["forbidden"] = terms
        for term in terms:
            if contains_marker(text, term):
                failures.append(f"{rel}: forbidden ann sparse-scope claim: {term}")
    return {
        "schema_version": "cortexdb.ann_sparse_scope_recall.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": checked,
        "invariant": (
            "fallback-enabled sparse allowed-set ANN search with an explicit visit budget "
            "routes to exact top-k before budget-starved scope-blind graph traversal"
        ),
        "boundary": {
            "proves": (
                "small allowed sets use exact top-k with fallback_reason=sparse_allowed_set "
                "instead of silently depending on a shared HNSW visit budget"
            ),
            "does_not_prove": (
                "fallback-disabled sparse ANN production eligibility or full per-scope "
                "ANN subgraph partitioning; dense allowed sets still use the normal HNSW "
                "budget/fallback reporting path"
            ),
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
    print(f"ann sparse-scope recall check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
