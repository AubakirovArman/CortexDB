#!/usr/bin/env python3
"""Validate investment_projects query and ground-truth files."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not raw.strip():
            continue
        value = json.loads(raw)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{line_no}: row must be an object")
        rows.append(value)
    return rows


def non_empty_str(row: dict, key: str, label: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label}: missing {key}")
    return value


def validate(documents: Path, chunks: Path, queries: Path, truth: Path, min_queries: int, min_truth: int) -> dict:
    doc_ids = {non_empty_str(row, "doc_id", f"{documents}:{idx}") for idx, row in enumerate(load_jsonl(documents), 1)}
    chunk_ids = {non_empty_str(row, "chunk_id", f"{chunks}:{idx}") for idx, row in enumerate(load_jsonl(chunks), 1)}
    query_rows = load_jsonl(queries)
    if len(query_rows) < min_queries:
        raise ValueError(f"expected at least {min_queries} queries, got {len(query_rows)}")
    query_ids: set[str] = set()
    for index, row in enumerate(query_rows, 1):
        query_id = non_empty_str(row, "query_id", f"{queries}:{index}")
        non_empty_str(row, "query", f"{queries}:{index}")
        non_empty_str(row, "intent", f"{queries}:{index}")
        non_empty_str(row, "name", f"{queries}:{index}")
        if query_id in query_ids:
            raise ValueError(f"{queries}:{index}: duplicate query_id {query_id}")
        query_ids.add(query_id)
    truth_rows = load_jsonl(truth)
    covered = 0
    for index, row in enumerate(truth_rows, 1):
        label = f"{truth}:{index}"
        query_id = non_empty_str(row, "query_id", label)
        if query_id not in query_ids:
            raise ValueError(f"{label}: unknown query_id {query_id}")
        relevant_docs = row.get("relevant_doc_ids")
        relevant_chunks = row.get("relevant_chunk_ids")
        if not isinstance(relevant_docs, list) or not relevant_docs:
            raise ValueError(f"{label}: relevant_doc_ids must be non-empty")
        if not isinstance(relevant_chunks, list) or not relevant_chunks:
            raise ValueError(f"{label}: relevant_chunk_ids must be non-empty")
        for doc_id in relevant_docs:
            if doc_id not in doc_ids:
                raise ValueError(f"{label}: unknown relevant doc {doc_id}")
        for chunk_id in relevant_chunks:
            if chunk_id not in chunk_ids:
                raise ValueError(f"{label}: unknown relevant chunk {chunk_id}")
        covered += 1
    if covered < min_truth:
        raise ValueError(f"expected ground truth for at least {min_truth} queries, got {covered}")
    return {"ok": True, "queries": len(query_rows), "ground_truth": covered}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--documents", type=Path, default=Path("corpus/documents.jsonl"))
    parser.add_argument("--chunks", type=Path, default=Path("corpus/chunks.jsonl"))
    parser.add_argument("--queries", type=Path, default=Path("queries/queries.jsonl"))
    parser.add_argument("--ground-truth", type=Path, default=Path("queries/ground_truth.jsonl"))
    parser.add_argument("--min-queries", type=int, default=40)
    parser.add_argument("--min-truth", type=int, default=20)
    args = parser.parse_args()
    print(json.dumps(validate(args.documents, args.chunks, args.queries, args.ground_truth, args.min_queries, args.min_truth), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
