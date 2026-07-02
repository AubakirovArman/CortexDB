#!/usr/bin/env python3
"""Validate the FC-7 fail-closed invariant model wiring."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-aql/tests/fail_closed_invariant_model.rs": [
        "bitmap_program_respects_fail_closed_model_over_randomized_catalog_views",
        "assert_bitmap_program_model",
    ],
    "crates/cortex-aql/tests/fail_closed_invariant_model/fixture.rs": [
        "MODEL_CASES: usize = 128",
        "BitmapOp::PushAgentAllowed",
        "BitmapOp::PushLive",
        "BitmapOp::And",
        "positive_cases",
        "admitted.is_subset(&spec)",
    ],
    "crates/cortex-aql/tests/fail_closed_invariant_model/helpers.rs": [
        "DeterministicRng",
        "wrapping_mul(6_364_136_223_846_793_005)",
    ],
    "crates/cortex-engine/tests/fail_closed_invariant_model.rs": [
        "fail_closed_invariant_model_hash_is_stable",
    ],
    "crates/cortex-engine/src/search/database/fail_closed_invariant_model_tests.rs": [
        "persisted_ann_and_lexical_paths_respect_fail_closed_model",
        "ENGINE_MODEL_CASES: usize = 32",
        "SearchMode::Keyword",
        "SearchMode::Vector",
        "SearchMode::Hybrid",
        "context_pack_from_aql",
        "search_cells_with_bound_retrieve_plan",
    ],
    "crates/cortex-engine/src/search/database.rs": [
        "mod fail_closed_invariant_model_tests;",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "FAIL_CLOSED_INVARIANT_MODEL_SCHEMA",
        "FAIL_CLOSED_INVARIANT_MODEL_HASH_DOMAIN",
        "FAIL_CLOSED_INVARIANT_MODEL_HASH",
        "FAIL_CLOSED_INVARIANT_MODEL_SPEC",
        "fail_closed_invariant_model_hash",
        "allowed_cells_from_bound_retrieve_plan(plan)",
    ],
    "mk/core-retrieval-context.mk": [
        "fail-closed-invariant-model-check:",
        "cargo test -p cortex-aql --test fail_closed_invariant_model",
        "cargo test -p cortex-engine fail_closed_invariant_model_tests::persisted_ann_and_lexical_paths_respect_fail_closed_model --all-features",
        "cargo test -p cortex-engine --test fail_closed_invariant_model --all-features -- --nocapture",
        'python3 scripts/fail_closed_invariant_model_check.py --root "." --report "$(FAIL_CLOSED_INVARIANT_MODEL_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "FAIL_CLOSED_INVARIANT_MODEL_REPORT ?= target/fail-closed-invariant-model/report.json",
    ],
    "mk/phony.mk": [
        "fail-closed-invariant-model-check",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "FC-7",
        "fail-closed-invariant-model-check",
        "model_hash",
    ],
}


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def extract_model_hash(accountability_text: str) -> str | None:
    match = re.search(
        r'FAIL_CLOSED_INVARIANT_MODEL_HASH:\s*&str\s*=\s*"([0-9a-f]{64})"',
        accountability_text,
    )
    return match.group(1) if match else None


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, list[str]] = {}
    accountability_text = ""
    for rel, terms in REQUIRED_TERMS.items():
        try:
            text = read_text(root, rel)
        except FileNotFoundError:
            failures.append(f"{rel}: missing file")
            checked[rel] = terms
            continue
        if rel == "crates/cortex-engine/src/accountability.rs":
            accountability_text = text
        checked[rel] = terms
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing fail-closed invariant marker: {term}")

    model_hash = extract_model_hash(accountability_text)
    if model_hash is None:
        failures.append("crates/cortex-engine/src/accountability.rs: missing pinned 64-hex model_hash")

    return {
        "schema_version": "cortexdb.fail_closed_invariant_model.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "model_hash": model_hash,
        "aql_randomized_cases": 128,
        "engine_randomized_cases": 32,
        "checked": checked,
        "invariant": "admitted_set subseteq agent_allowed intersect live intersect where",
        "paths": ["bitmap_program", "context_pack_aql", "persisted_keyword", "persisted_vector", "persisted_hybrid"],
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
    print(f"fail-closed invariant model check passed: {report_path}")
    print(f"model_hash={report['model_hash']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
