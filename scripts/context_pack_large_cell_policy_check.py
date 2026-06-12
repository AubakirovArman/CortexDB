#!/usr/bin/env python3
"""Validate ContextPack large-cell policy evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_TERMS = {
    "crates/cortex-engine/src/context/large_cell.rs": [
        "ContextLargeCellPolicy",
        "Truncate",
        "Exclude",
        "SummarizePlaceholder",
        "SourceOnlyReference",
        "apply_large_cell_policy",
    ],
    "crates/cortex-engine/tests/context_pack_large_cell_policy.rs": [
        "truncate_policy_keeps_prefix_within_budget",
        "exclude_policy_drops_oversized_cell",
        "summarize_placeholder_policy_keeps_deterministic_reference",
        "source_only_reference_policy_keeps_provenance_without_body",
    ],
    "docs/CONTEXT_PACK.md": [
        "ContextLargeCellPolicy",
        "truncate",
        "summarize-placeholder",
        "source-only reference",
        "context-pack-large-cell-policy-check",
    ],
    "docs/archive/CONTEXT_PACK_TECHNOLOGY.md": [
        "Large Cell Policy",
        "source-only reference",
    ],
    "docs/archive/CONTEXT_PACK_QUALITY_EVIDENCE.md": [
        "Large Cell Policy Evidence",
        "context_pack_large_cell_policy",
    ],
    "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md": [
        "Epic 69. ContextPack Large Cell Policy",
        "Status: done",
    ],
}


def validate(root: Path) -> list[dict[str, object]]:
    results: list[dict[str, object]] = []
    for relative, terms in REQUIRED_TERMS.items():
        path = root / relative
        if not path.exists():
            raise SystemExit(f"missing required file: {relative}")
        text = path.read_text(encoding="utf-8")
        missing = [term for term in terms if term not in text]
        if missing:
            raise SystemExit(f"{relative}: missing terms: {', '.join(missing)}")
        results.append(
            {
                "path": relative,
                "checked_terms": terms,
                "status": "ok",
            }
        )
    return results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    args = parser.parse_args()

    report = Path(args.report)
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(
        json.dumps(
            {
                "gate": "context_pack_large_cell_policy",
                "status": "passed",
                "checks": validate(Path(args.root)),
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"context pack large cell policy passed: {report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
