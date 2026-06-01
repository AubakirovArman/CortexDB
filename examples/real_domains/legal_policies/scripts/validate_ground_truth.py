#!/usr/bin/env python3
"""Validate legal_policies queries and ground-truth references."""

from __future__ import annotations

import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = ROOT / "corpus" / "documents.jsonl"
CHUNKS = ROOT / "corpus" / "chunks.jsonl"
QUERIES = ROOT / "queries" / "queries.jsonl"
GROUND_TRUTH = ROOT / "queries" / "ground_truth.jsonl"


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number}: expected object")
            rows.append(value)
    return rows


def main() -> int:
    docs = {row["doc_id"] for row in load_jsonl(DOCUMENTS)}
    chunks = {row["chunk_id"] for row in load_jsonl(CHUNKS)}
    queries = load_jsonl(QUERIES)
    query_ids = set()
    for row in queries:
        for field in ["query_id", "name", "query", "intent"]:
            if not isinstance(row.get(field), str) or not row[field].strip():
                raise ValueError(f"{row.get('query_id', '<missing-query-id>')}: missing {field}")
        if row["query_id"] in query_ids:
            raise ValueError(f"duplicate query_id: {row['query_id']}")
        query_ids.add(row["query_id"])

    truth_rows = load_jsonl(GROUND_TRUTH)
    for row in truth_rows:
        query_id = row.get("query_id")
        if query_id not in query_ids:
            raise ValueError(f"unknown query_id in ground truth: {query_id}")
        for doc_id in row.get("relevant_doc_ids", []):
            if doc_id not in docs:
                raise ValueError(f"{query_id}: unknown relevant doc_id {doc_id}")
        for chunk_id in row.get("relevant_chunk_ids", []):
            if chunk_id not in chunks:
                raise ValueError(f"{query_id}: unknown relevant chunk_id {chunk_id}")

    print(f"legal_policies ground truth valid: queries={len(query_ids)} rows={len(truth_rows)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"legal_policies ground truth validation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
