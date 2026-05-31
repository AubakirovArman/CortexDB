#!/usr/bin/env python3
"""Regenerate chunks.jsonl from documents.jsonl."""

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


def chunk_doc(doc: dict, size: int, overlap: int, min_size: int) -> list[dict]:
    text = doc.get("text")
    if not isinstance(text, str) or not text.strip():
        raise ValueError(f"{doc.get('doc_id', '<unknown>')}: missing text")
    chunks = []
    start = 0
    index = 1
    while start < len(text):
        part = text[start:start + size].strip()
        if len(part) >= min_size:
            chunks.append({
                "chunk_id": f"{doc['doc_id']}_c{index:03d}",
                "doc_id": doc["doc_id"],
                "source": doc["source"],
                "country": doc["country"],
                "sector": doc["sector"],
                "title": doc["title"],
                "text": part,
                "payload": part,
            })
            index += 1
        if start + size >= len(text):
            break
        start += size - overlap
    return chunks


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--documents", type=Path, default=Path("corpus/documents.jsonl"))
    parser.add_argument("--output", type=Path, default=Path("corpus/chunks.jsonl"))
    parser.add_argument("--chunk-size", type=int, default=800)
    parser.add_argument("--overlap", type=int, default=120)
    parser.add_argument("--min-chunk-size", type=int, default=100)
    args = parser.parse_args()
    if args.chunk_size <= args.overlap:
        raise ValueError("--chunk-size must be larger than --overlap")
    chunks = []
    for doc in load_jsonl(args.documents):
        chunks.extend(chunk_doc(doc, args.chunk_size, args.overlap, args.min_chunk_size))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in chunks),
        encoding="utf-8",
    )
    print(f"chunks={len(chunks)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
