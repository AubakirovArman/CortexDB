#!/usr/bin/env python3
"""Build a deterministic balanced LongMemEval retrieval/reference subset."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def allocate_counts(groups: dict[str, list[dict[str, Any]]], limit: int) -> dict[str, int]:
    if limit <= 0:
        raise ValueError("limit must be positive")
    total = sum(len(rows) for rows in groups.values())
    if total == 0:
        raise ValueError("reference file has no rows")
    if limit >= total:
        return {name: len(rows) for name, rows in groups.items()}

    allocation: dict[str, int] = {}
    remainders: list[tuple[float, str]] = []
    used = 0
    for name, rows in groups.items():
        raw = len(rows) * limit / total
        count = int(raw)
        if count == 0 and rows and limit >= len(groups):
            count = 1
        allocation[name] = min(count, len(rows))
        used += allocation[name]
        remainders.append((raw - int(raw), name))

    for _, name in sorted(remainders, reverse=True):
        if used >= limit:
            break
        if allocation[name] < len(groups[name]):
            allocation[name] += 1
            used += 1

    for _, name in sorted(remainders):
        if used <= limit:
            break
        if allocation[name] > 0:
            allocation[name] -= 1
            used -= 1

    return allocation


def sample_evenly(rows: list[dict[str, Any]], count: int) -> list[dict[str, Any]]:
    if count <= 0:
        return []
    if count >= len(rows):
        return list(rows)
    if count == 1:
        return [rows[0]]

    selected: list[dict[str, Any]] = []
    seen: set[int] = set()
    for index in range(count):
        source_index = round(index * (len(rows) - 1) / (count - 1))
        while source_index in seen and source_index + 1 < len(rows):
            source_index += 1
        while source_index in seen and source_index > 0:
            source_index -= 1
        seen.add(source_index)
        selected.append(rows[source_index])
    return selected


def run(args: argparse.Namespace) -> dict[str, Any]:
    refs = read_json(args.reference_file)
    retrieval_rows = read_jsonl(args.retrieval_log)
    retrieval_by_id = {row["question_id"]: row for row in retrieval_rows}

    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in refs:
        groups[row["question_type"]].append(row)

    allocation = allocate_counts(dict(groups), args.limit)
    selected_refs: list[dict[str, Any]] = []
    for question_type in sorted(groups):
        selected_refs.extend(sample_evenly(groups[question_type], allocation[question_type]))

    selected_ids = {row["question_id"] for row in selected_refs}
    missing = sorted(selected_ids - set(retrieval_by_id))
    if missing:
        raise RuntimeError(f"missing retrieval rows for {len(missing)} question IDs: {missing[:10]}")

    selected_refs_by_id = {row["question_id"]: row for row in selected_refs}
    ordered_retrieval = [row for row in retrieval_rows if row["question_id"] in selected_ids]
    ordered_refs = [selected_refs_by_id[row["question_id"]] for row in ordered_retrieval]

    reference_out = args.output_root / f"{args.output_prefix}_reference.json"
    retrieval_out = args.output_root / f"{args.output_prefix}_retrieval.jsonl"
    write_json(reference_out, ordered_refs)
    write_jsonl(retrieval_out, ordered_retrieval)

    actual_by_type: dict[str, int] = defaultdict(int)
    for row in ordered_refs:
        actual_by_type[row["question_type"]] += 1

    report = {
        "schema_version": "cortexdb.longmemeval.v1.balanced_subset.v1",
        "limit": args.limit,
        "questions": len(ordered_refs),
        "retrieval_rows": len(ordered_retrieval),
        "by_question_type": dict(sorted(actual_by_type.items())),
        "reference_file": str(reference_out),
        "retrieval_log": str(retrieval_out),
    }
    write_json(args.output_root / f"{args.output_prefix}_subset_report.json", report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-file", type=Path, required=True)
    parser.add_argument("--retrieval-log", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--output-prefix", default="balanced")
    return parser.parse_args()


def main() -> int:
    print(json.dumps(run(parse_args()), ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
