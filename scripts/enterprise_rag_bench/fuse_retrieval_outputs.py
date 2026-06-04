#!/usr/bin/env python3
"""Fuse EnterpriseRAG-Bench retrieval outputs with reciprocal-rank fusion."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def rows_by_question_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in indexed:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        indexed[qid] = row
    return indexed


def parse_weight(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("--weight values must be numeric") from error
    if parsed <= 0:
        raise argparse.ArgumentTypeError("--weight values must be positive")
    return parsed


def fuse_doc_ids(rows: list[dict[str, Any]], weights: list[float], rrf_k: int, limit: int) -> list[str]:
    scores: dict[str, float] = {}
    first_seen: dict[str, int] = {}
    order = 0
    for source_index, row in enumerate(rows):
        weight = weights[source_index]
        for rank, doc_id in enumerate(row.get("document_ids", []), 1):
            if not isinstance(doc_id, str) or not doc_id:
                continue
            order += 1
            first_seen.setdefault(doc_id, order)
            scores[doc_id] = scores.get(doc_id, 0.0) + weight / float(rrf_k + rank)
    return [
        doc_id
        for doc_id, _ in sorted(
            scores.items(),
            key=lambda item: (-item[1], first_seen[item[0]], item[0]),
        )[:limit]
    ]


def run(args: argparse.Namespace) -> dict[str, Any]:
    if len(args.input) < 2:
        raise ValueError("at least two --input files are required")
    weights = args.weight or [1.0] * len(args.input)
    if len(weights) != len(args.input):
        raise ValueError("--weight count must match --input count")

    sources = [rows_by_question_id(read_jsonl(path), str(path)) for path in args.input]
    qids = list(sources[0].keys())
    for path, source in zip(args.input[1:], sources[1:]):
        if set(source) != set(qids):
            raise ValueError(f"{path} question_id set does not match first input")

    output_rows: list[dict[str, Any]] = []
    changed = 0
    for qid in qids:
        source_rows = [source[qid] for source in sources]
        fused = fuse_doc_ids(source_rows, weights, args.rrf_k, args.limit)
        row = dict(source_rows[0])
        if fused != row.get("document_ids", []):
            changed += 1
        row["document_ids"] = fused
        row["fusion"] = {
            "method": "rrf",
            "inputs": [str(path) for path in args.input],
            "weights": weights,
            "rrf_k": args.rrf_k,
        }
        output_rows.append(row)

    write_jsonl(args.output, output_rows)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_fusion_report.v1",
        "questions": len(output_rows),
        "inputs": [str(path) for path in args.input],
        "output": str(args.output),
        "weights": weights,
        "rrf_k": args.rrf_k,
        "limit": args.limit,
        "changed_rows": changed,
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--weight", type=parse_weight, action="append")
    parser.add_argument("--rrf-k", type=int, default=60)
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    if args.rrf_k <= 0:
        parser.error("--rrf-k must be positive")
    if args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
