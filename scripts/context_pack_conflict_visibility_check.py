#!/usr/bin/env python3
"""Validate the ContextPack conflict visibility contract evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "engine structs": [
        "conflict_visibility_q16",
        "visible_conflict_count",
    ],
    "conflict module": [
        "ContextConflictVisibility",
        "visible_conflict_count",
        "extract_project_metric_value",
    ],
    "engine tests": [
        "conflict_visibility_is_zero_without_conflicting_values",
        "conflict_visibility_reports_conflicting_project_metric_values",
        "conflict_visibility_counts_distinct_conflict_groups",
        "conflict_visibility_is_exported_in_json_prompt_and_markdown",
    ],
    "public json": [
        "conflict_visibility_q16",
        "visible_conflict_count",
    ],
    "docs": [
        "conflict_visibility_q16",
        "visible_conflict_count",
        "make context-pack-conflict-visibility-check",
    ],
}


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(name: str, text: str, terms: list[str]) -> list[str]:
    return [f"{name}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    files = {
        "engine structs": read_text(root / "crates/cortex-engine/src/context/mod.rs"),
        "conflict module": read_text(root / "crates/cortex-engine/src/context/conflicts.rs"),
        "engine tests": read_text(
            root / "crates/cortex-engine/tests/context_pack_conflict_visibility.rs"
        ),
        "public json": "\n".join(
            [
                read_text(root / "crates/cortex-engine/src/context/export/json_export.rs"),
                read_text(root / "crates/cortex-server/src/responses.rs"),
                read_text(root / "crates/cortex-cli/src/cli_json_types.rs"),
                read_text(root / "crates/cortex-sdk/src/types.rs"),
                read_text(root / "docs/openapi.yaml"),
            ]
        ),
        "docs": "\n".join(
            [
                read_text(root / "docs/CONTEXT_PACK.md"),
                read_text(root / "docs/archive/CONTEXT_PACK_TECHNOLOGY.md"),
                read_text(root / "docs/archive/CONTEXT_PACK_QUALITY_EVIDENCE.md"),
                read_text(root / "docs/API_JSON_SCHEMAS.md"),
            ]
        ),
    }

    failures: list[str] = []
    for name, terms in REQUIRED_TERMS.items():
        failures.extend(missing_terms(name, files[name], terms))

    return {
        "schema_version": "context_pack_conflict_visibility.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checks": REQUIRED_TERMS,
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
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"context pack conflict visibility passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
