#!/usr/bin/env python3
"""Validate bound-plan persisted ANN/lexical allowed-set parity wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-engine/src/search/database/allowed.rs": [
        "allowed_cells_from_bound_retrieve_plan",
        "eval_bitmap_program(&plan.bitmap_program, &provider)",
        "provider.cell_id_for_candidate(candidate)",
        "persisted_allowed_candidates",
        "allowed.retain",
        "candidate_to_cell",
        "allowed_cells.contains(cell_id)",
    ],
    "crates/cortex-engine/src/search/database/api.rs": [
        "search_cells_with_bound_retrieve_plan",
        "allowed_cells_from_bound_retrieve_plan(self, view, plan)",
        "search_cells_with_report_with_policy_and_allowed_cells",
    ],
    "crates/cortex-engine/src/search/database/persisted.rs": [
        "allowed_cells: Option<&std::collections::BTreeSet<CellId>>",
        "persisted_allowed_candidates(&state, view, allowed_cells)",
        "search_persisted_lexical",
        "search_persisted_ann",
        "search_disk_resident_vectors",
    ],
    "crates/cortex-engine/src/search/database/snapshot.rs": [
        "allowed_cells: Option<&BTreeSet<CellId>>",
        "snapshot_search_records(view, query, allowed_cells)",
        "records.retain(|record| allowed_cells.contains(&record.cell_id))",
        "!allowed.contains(&version.cell_id)",
    ],
    "crates/cortex-engine/src/search/database/tests.rs": [
        "persisted_search_bound_plan_allowed_set_filters_status_and_where",
        "status=draft",
        "type=document_block",
        "legacy_vector",
        "BoundPlan::Retrieve(plan)",
        "SearchMode::Keyword",
        "SearchMode::Vector",
        "SearchMode::Hybrid",
    ],
    "mk/core-retrieval-context.mk": [
        "ann-scope-parity-check:",
        "cargo test -p cortex-engine search::database::tests::persisted_search_bound_plan_allowed_set_filters_status_and_where --all-features",
        'python3 scripts/ann_scope_parity_check.py --root "." --report "$(ANN_SCOPE_PARITY_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "ANN_SCOPE_PARITY_REPORT ?= target/ann-scope-parity/report.json",
    ],
    "mk/phony.mk": [
        "ann-scope-parity-check",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "ann-scope-parity-check",
        "bound bitmap program",
        "persisted ANN/lexical",
        "FC-2",
    ],
}

FORBIDDEN_TERMS = {
    "scripts/context_access_decision_capture_check.py": [
        "ann-scope-parity-check",
        "persisted ANN/lexical",
        "bound bitmap program",
    ],
    "scripts/segment_pruning_parity_boundary_check.py": [
        "production_safe\": true",
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
                failures.append(f"{rel}: missing ann-scope parity marker: {term}")
    for rel, terms in FORBIDDEN_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["forbidden"] = terms
        for term in terms:
            if contains_marker(text, term):
                failures.append(f"{rel}: forbidden ann-scope parity claim marker: {term}")
    return {
        "schema_version": "cortexdb.ann_scope_parity.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": checked,
        "invariant": (
            "persisted ANN/vector and lexical search can be constrained by cells "
            "derived from the bound retrieve bitmap program"
        ),
        "boundary": {
            "proves": "bound-plan persisted search allowed basis excludes readable cells rejected by status or WHERE",
            "does_not_prove": "scope-blind /v1/search requests without a bound retrieve plan apply arbitrary AQL WHERE clauses",
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
    print(f"ann-scope parity check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
