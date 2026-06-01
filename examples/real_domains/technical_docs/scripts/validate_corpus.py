#!/usr/bin/env python3
"""Validate the technical_docs retrieval corpus."""

from __future__ import annotations

import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = ROOT / "corpus" / "documents.jsonl"
CHUNKS = ROOT / "corpus" / "chunks.jsonl"


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


def require_text(row: dict, field: str, label: str) -> None:
    if not isinstance(row.get(field), str) or not row[field].strip():
        raise ValueError(f"{label}: missing {field}")


def main() -> int:
    docs = load_jsonl(DOCUMENTS)
    chunks = load_jsonl(CHUNKS)
    doc_ids = set()
    chunk_ids = set()

    for row in docs:
        label = row.get("doc_id", "<missing-doc-id>")
        for field in ["doc_id", "source", "component", "title", "version", "url", "text"]:
            require_text(row, field, str(label))
        if row["doc_id"] in doc_ids:
            raise ValueError(f"duplicate doc_id: {row['doc_id']}")
        doc_ids.add(row["doc_id"])

    for row in chunks:
        label = row.get("chunk_id", "<missing-chunk-id>")
        for field in ["chunk_id", "doc_id", "source", "component", "version", "title", "text"]:
            require_text(row, field, str(label))
        if row["chunk_id"] in chunk_ids:
            raise ValueError(f"duplicate chunk_id: {row['chunk_id']}")
        if row["doc_id"] not in doc_ids:
            raise ValueError(f"{label}: unknown doc_id {row['doc_id']}")
        chunk_ids.add(row["chunk_id"])

    print(f"technical_docs corpus valid: documents={len(docs)} chunks={len(chunks)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"technical_docs corpus validation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
