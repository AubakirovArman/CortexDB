#!/usr/bin/env python3
"""Validate investment_projects documents/chunks corpus files."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_DOC_FIELDS = ("doc_id", "source", "country", "title", "sector", "url", "text")
REQUIRED_CHUNK_FIELDS = ("chunk_id", "doc_id", "source", "country", "sector", "title", "text")


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


def require_fields(row: dict, fields: tuple[str, ...], label: str) -> None:
    for field in fields:
        value = row.get(field)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"{label}: missing {field}")


def validate(documents: Path, chunks: Path, min_documents: int, min_chunks: int) -> dict:
    docs = load_jsonl(documents)
    chunk_rows = load_jsonl(chunks)
    if len(docs) < min_documents:
        raise ValueError(f"expected at least {min_documents} documents, got {len(docs)}")
    if len(chunk_rows) < min_chunks:
        raise ValueError(f"expected at least {min_chunks} chunks, got {len(chunk_rows)}")
    doc_ids: set[str] = set()
    for index, row in enumerate(docs, 1):
        require_fields(row, REQUIRED_DOC_FIELDS, f"{documents}:{index}")
        doc_id = row["doc_id"]
        if doc_id in doc_ids:
            raise ValueError(f"{documents}:{index}: duplicate doc_id {doc_id}")
        doc_ids.add(doc_id)
    chunk_ids: set[str] = set()
    lengths = []
    for index, row in enumerate(chunk_rows, 1):
        require_fields(row, REQUIRED_CHUNK_FIELDS, f"{chunks}:{index}")
        chunk_id = row["chunk_id"]
        if chunk_id in chunk_ids:
            raise ValueError(f"{chunks}:{index}: duplicate chunk_id {chunk_id}")
        if row["doc_id"] not in doc_ids:
            raise ValueError(f"{chunks}:{index}: unknown doc_id {row['doc_id']}")
        chunk_ids.add(chunk_id)
        lengths.append(len(row["text"]))
    return {
        "ok": True,
        "documents": len(docs),
        "chunks": len(chunk_rows),
        "avg_chunk_chars": sum(lengths) // max(1, len(lengths)),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--documents", type=Path, default=Path("corpus/documents.jsonl"))
    parser.add_argument("--chunks", type=Path, default=Path("corpus/chunks.jsonl"))
    parser.add_argument("--min-documents", type=int, default=20)
    parser.add_argument("--min-chunks", type=int, default=150)
    args = parser.parse_args()
    print(json.dumps(validate(args.documents, args.chunks, args.min_documents, args.min_chunks), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
