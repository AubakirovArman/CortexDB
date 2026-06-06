#!/usr/bin/env python3
"""Verify the Deterministic Chunking v1 contract is wired across the repo."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


CHECKS = [
    (
        "text_overlap_policy",
        "crates/cortex-engine/src/ingestion/chunking.rs",
        ["TextOverlapPolicy", "FixedChars", "overlap_policy"],
    ),
    (
        "json_chunk_policy",
        "crates/cortex-engine/src/ingestion/chunking.rs",
        ["JsonChunkPolicy", "DEFAULT_JSON_CHUNK_PATH_SEPARATOR", "join_path"],
    ),
    (
        "table_chunk_policy",
        "crates/cortex-engine/src/ingestion/chunking.rs",
        ["TableChunkPolicy", "source_row_number", "cell_range"],
    ),
    (
        "json_sorted_paths",
        "crates/cortex-engine/src/ingestion/formats.rs",
        ["flat_json_fields_with_policy", "policy.sort_paths", "out.sort_by"],
    ),
    (
        "csv_uses_table_policy",
        "crates/cortex-engine/src/ingestion/adapters.rs",
        ["TableChunkPolicy::default", "source_row_number", "cell_range"],
    ),
    (
        "public_policy_exports",
        "crates/cortex-engine/src/ingestion.rs",
        ["JsonChunkPolicy", "TableChunkPolicy", "TextOverlapPolicy"],
    ),
    (
        "policy_tests",
        "crates/cortex-engine/tests/ingestion_chunking_policy.rs",
        [
            "text_chunk_ids_and_overlap_are_deterministic",
            "json_ingestion_uses_sorted_leaf_paths_as_policy",
            "table_policy_uses_one_based_source_rows_and_cell_ranges",
        ],
    ),
    (
        "policy_docs",
        "docs/DETERMINISTIC_CHUNKING.md",
        ["TextOverlapPolicy::FixedChars", "JsonChunkPolicy", "TableChunkPolicy"],
    ),
    (
        "ingestion_docs_link_policy",
        "docs/INGESTION.md",
        ["DETERMINISTIC_CHUNKING.md", "JSON emits sorted", "CSV/table ingestion"],
    ),
]


def read_text(relative_path: str) -> str:
    return (REPO_ROOT / relative_path).read_text(encoding="utf-8")


def run_checks() -> tuple[bool, list[dict[str, object]]]:
    results = []
    ok = True
    for name, relative_path, markers in CHECKS:
        text = read_text(relative_path)
        missing = [marker for marker in markers if marker not in text]
        passed = not missing
        ok = ok and passed
        results.append(
            {
                "name": name,
                "path": relative_path,
                "passed": passed,
                "missing": missing,
            }
        )
    return ok, results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, default=None)
    args = parser.parse_args()

    ok, checks = run_checks()
    report = {
        "deterministic_chunking_v1": ok,
        "checks": checks,
    }
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
