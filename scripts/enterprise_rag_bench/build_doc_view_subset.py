#!/usr/bin/env python3
"""Build multi-view records only for documents seen in retrieval candidates."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from build_doc_views import (
    chunk_views,
    entity_view,
    metadata_view,
    normalize_ws,
    source_type,
    summary_view,
)
from multi_index_candidate_generation import extract_document_content


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def candidate_doc_ids(paths: list[Path], limit: int | None) -> list[str]:
    seen: set[str] = set()
    values: list[str] = []
    for path in paths:
        for row in read_jsonl(path):
            docs = [str(item) for item in row.get("document_ids", []) if str(item)]
            for doc_id in docs[:limit]:
                if doc_id in seen:
                    continue
                seen.add(doc_id)
                values.append(doc_id)
    return values


def build_row(doc_id: str, rel_path: str, document: dict[str, Any], args: argparse.Namespace) -> dict[str, Any]:
    title, content = extract_document_content(document)
    return {
        "doc_id": doc_id,
        "source_type": source_type(rel_path),
        "path": rel_path,
        "title_view": normalize_ws(title)[: args.max_view_chars],
        "path_view": normalize_ws(rel_path.replace("/", " ").replace("-", " ").replace("_", " ")),
        "body_view": normalize_ws(content[: args.max_body_chars]),
        "source_metadata_view": metadata_view(document, max_chars=args.max_view_chars),
        "entity_view": entity_view(document, rel_path, title),
        "summary_view": summary_view(document, title, content, max_chars=args.max_view_chars),
        "chunk_views": chunk_views(
            content,
            chunk_chars=args.chunk_chars,
            overlap_chars=args.chunk_overlap_chars,
            max_chunks=args.max_chunks_per_doc,
        ),
    }


def run(args: argparse.Namespace) -> dict[str, Any]:
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    doc_ids = candidate_doc_ids(args.retrieval_file, args.candidate_limit)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    skipped = 0
    written = 0
    source_counts: dict[str, int] = {}
    with args.output.open("w", encoding="utf-8") as handle:
        for doc_id in doc_ids:
            rel_path = str(uuid_index.get(doc_id, ""))
            if not rel_path:
                skipped += 1
                continue
            try:
                document = read_json(args.sources_dir / rel_path)
            except (OSError, UnicodeDecodeError, json.JSONDecodeError):
                skipped += 1
                continue
            if not isinstance(document, dict):
                skipped += 1
                continue
            row = build_row(doc_id, rel_path, document, args)
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")
            source_counts[str(row["source_type"])] = source_counts.get(str(row["source_type"]), 0) + 1
            written += 1
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.doc_view_subset.v1",
        "retrieval_files": [str(path) for path in args.retrieval_file],
        "uuid_index": str(args.uuid_index),
        "sources_dir": str(args.sources_dir),
        "output": str(args.output),
        "candidate_limit": args.candidate_limit,
        "candidate_doc_ids": len(doc_ids),
        "rows_written": written,
        "skipped": skipped,
        "source_counts": dict(sorted(source_counts.items())),
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, action="append", required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--candidate-limit", type=int, default=120)
    parser.add_argument("--max-view-chars", type=int, default=1200)
    parser.add_argument("--max-body-chars", type=int, default=2200)
    parser.add_argument("--chunk-chars", type=int, default=900)
    parser.add_argument("--chunk-overlap-chars", type=int, default=120)
    parser.add_argument("--max-chunks-per-doc", type=int, default=4)
    args = parser.parse_args()
    for name in ("candidate_limit", "max_view_chars", "max_body_chars", "chunk_chars", "max_chunks_per_doc"):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.chunk_overlap_chars < 0:
        parser.error("--chunk-overlap-chars must be non-negative")
    return args


def main() -> int:
    report = run(parse_args())
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
