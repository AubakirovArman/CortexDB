#!/usr/bin/env python3
"""Build a deterministic EnterpriseRAG-Bench question subset."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                value = json.loads(line)
                if isinstance(value, dict):
                    rows.append(value)
    return rows


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")


def allocate_counts(groups: dict[str, list[dict[str, Any]]], limit: int) -> dict[str, int]:
    if limit <= 0:
        raise ValueError("limit must be positive")
    total = sum(len(rows) for rows in groups.values())
    if limit >= total:
        return {name: len(rows) for name, rows in groups.items()}

    used = 0
    allocation: dict[str, int] = {}
    remainders: list[tuple[float, str]] = []
    for name, rows in groups.items():
        raw = len(rows) * limit / total
        count = int(raw)
        if count == 0 and limit >= len(groups):
            count = 1
        count = min(count, len(rows))
        allocation[name] = count
        used += count
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


def sort_key(row: dict[str, Any]) -> tuple[int, str, int]:
    qid = str(row.get("question_id", ""))
    prefix = "".join(ch for ch in qid if not ch.isdigit())
    digits = "".join(ch for ch in qid if ch.isdigit())
    return (0, prefix, int(digits)) if digits else (1, qid, 0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--output-prefix", default="balanced_50")
    args = parser.parse_args()

    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in read_jsonl(args.questions_file):
        groups[str(row.get("question_type", "unknown"))].append(row)

    allocation = allocate_counts(dict(groups), args.limit)
    selected: list[dict[str, Any]] = []
    for question_type in sorted(groups):
        selected.extend(sample_evenly(groups[question_type], allocation[question_type]))
    selected.sort(key=sort_key)

    output_root = args.output_root / args.output_prefix
    questions_path = output_root / f"{args.output_prefix}_questions.jsonl"
    report_path = output_root / f"{args.output_prefix}_subset_report.json"
    write_jsonl(questions_path, selected)
    by_type = Counter(str(row.get("question_type", "unknown")) for row in selected)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.balanced_subset.v1",
        "limit": args.limit,
        "questions": len(selected),
        "by_question_type": dict(sorted(by_type.items())),
        "questions_jsonl": str(questions_path),
    }
    write_json(report_path, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
