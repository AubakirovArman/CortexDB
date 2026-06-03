#!/usr/bin/env python3
"""Build a deterministic balanced MultiHop-RAG query subset."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def read_list(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, list):
        raise ValueError(f"{path}: expected JSON list")
    return [row for row in value if isinstance(row, dict)]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True, ensure_ascii=True) + "\n")


def allocate_counts(groups: dict[str, list[dict[str, Any]]], limit: int) -> dict[str, int]:
    if limit <= 0:
        raise ValueError("limit must be positive")
    total = sum(len(rows) for rows in groups.values())
    if total == 0:
        raise ValueError("no rows to sample")
    if limit >= total:
        return {name: len(rows) for name, rows in groups.items()}
    allocation: dict[str, int] = {}
    remainders: list[tuple[float, str]] = []
    used = 0
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


def evidence_facts(row: dict[str, Any]) -> list[str]:
    facts: list[str] = []
    for evidence in row.get("evidence_list", []):
        if isinstance(evidence, dict) and isinstance(evidence.get("fact"), str):
            facts.append(evidence["fact"])
    return facts


def query_id(index: int) -> str:
    return f"multihop_{index + 1:04d}"


def build(args: argparse.Namespace) -> dict[str, Any]:
    rows = read_list(args.queries)
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for index, row in enumerate(rows):
        row = dict(row)
        row["_cortexdb_query_id"] = query_id(index)
        groups[str(row.get("question_type", "unknown"))].append(row)
    allocation = allocate_counts(dict(groups), args.limit)
    selected: list[dict[str, Any]] = []
    for question_type in sorted(groups):
        selected.extend(sample_evenly(groups[question_type], allocation[question_type]))
    selected.sort(key=lambda row: row["_cortexdb_query_id"])

    official_rows = []
    queries_jsonl = []
    ground_truth_jsonl = []
    for row in selected:
        official = dict(row)
        row_id = official.pop("_cortexdb_query_id")
        official_rows.append(official)
        queries_jsonl.append(
            {
                "query_id": row_id,
                "query": row.get("query", ""),
                "question_type": row.get("question_type", ""),
                "answer": row.get("answer", ""),
                "evidence_count": len(row.get("evidence_list", [])),
            }
        )
        ground_truth_jsonl.append(
            {
                "query_id": row_id,
                "answer": row.get("answer", ""),
                "question_type": row.get("question_type", ""),
                "evidence_facts": evidence_facts(row),
                "evidence_titles": [
                    evidence.get("title", "")
                    for evidence in row.get("evidence_list", [])
                    if isinstance(evidence, dict)
                ],
                "evidence_urls": [
                    evidence.get("url", "")
                    for evidence in row.get("evidence_list", [])
                    if isinstance(evidence, dict)
                ],
            }
        )

    output_root = args.output_root / args.output_prefix
    official_path = output_root / f"{args.output_prefix}_multihop.json"
    queries_path = output_root / f"{args.output_prefix}_queries.jsonl"
    ground_truth_path = output_root / f"{args.output_prefix}_ground_truth.jsonl"
    report_path = output_root / f"{args.output_prefix}_subset_report.json"
    write_json(official_path, official_rows)
    write_jsonl(queries_path, queries_jsonl)
    write_jsonl(ground_truth_path, ground_truth_jsonl)
    by_type = Counter(row.get("question_type", "unknown") for row in official_rows)
    report = {
        "schema_version": "cortexdb.multihop_rag.balanced_subset.v1",
        "limit": args.limit,
        "questions": len(official_rows),
        "by_question_type": dict(sorted(by_type.items())),
        "official_subset": str(official_path),
        "queries_jsonl": str(queries_path),
        "ground_truth_jsonl": str(ground_truth_path),
    }
    write_json(report_path, report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--queries", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--output-prefix", default="balanced_50")
    report = build(parser.parse_args())
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
