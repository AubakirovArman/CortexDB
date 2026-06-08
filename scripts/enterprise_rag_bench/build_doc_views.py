#!/usr/bin/env python3
"""Build deterministic multi-view document records for EnterpriseRAG-Bench."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from multi_index_candidate_generation import extract_document_content
from question_decomposition import tokens


NOISY_FIELDS = {
    "content_field_names",
    "dataset_doc_uuid",
    "dataset_noise_document",
    "title_field_name",
}

SUMMARY_FIELD_HINTS = {
    "summary",
    "current_plan_summary",
    "notes",
    "next_step",
    "status",
    "resolution",
    "impact",
    "blockers",
    "topics",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def normalize_ws(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip()


def source_type(path: str) -> str:
    return path.split("/", 1)[0] if path else "unknown"


def stringify(value: Any, *, max_chars: int) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return normalize_ws(value)[:max_chars]
    if isinstance(value, (int, float, bool)):
        return str(value)
    if isinstance(value, list):
        return normalize_ws(" ".join(stringify(item, max_chars=max_chars) for item in value))[:max_chars]
    if isinstance(value, dict):
        parts = []
        for key in sorted(value)[:20]:
            parts.append(f"{key}: {stringify(value[key], max_chars=max_chars // 4 or 1)}")
        return normalize_ws(" ".join(parts))[:max_chars]
    return normalize_ws(str(value))[:max_chars]


def metadata_view(document: dict[str, Any], *, max_chars: int) -> str:
    parts: list[str] = []
    content_fields = set(document.get("content_field_names") or [])
    title_field = str(document.get("title_field_name") or "")
    for key in sorted(document):
        if key in NOISY_FIELDS or key in content_fields or key == title_field:
            continue
        value = document[key]
        if isinstance(value, (dict, list, str, int, float, bool)):
            text = stringify(value, max_chars=240)
            if text:
                parts.append(f"{key}: {text}")
        if len(" ".join(parts)) > max_chars:
            break
    return normalize_ws(" ".join(parts))[:max_chars]


def entity_view(document: dict[str, Any], path: str, title: str) -> str:
    values = [path, title]
    for key in sorted(document):
        lower = key.lower()
        if any(marker in lower for marker in ("owner", "assignee", "company", "team", "repo", "key", "project", "thread", "channel", "region")):
            values.append(stringify(document[key], max_chars=240))
    entity_tokens = sorted(set(tokens(" ".join(values))), key=lambda item: (-len(item), item))
    return " ".join(entity_tokens[:80])


def summary_view(document: dict[str, Any], title: str, content: str, *, max_chars: int) -> str:
    parts = [title]
    for key in sorted(document):
        if key.lower() in SUMMARY_FIELD_HINTS:
            text = stringify(document[key], max_chars=max_chars // 3)
            if text:
                parts.append(f"{key}: {text}")
    if len(" ".join(parts)) < max_chars // 2:
        parts.append(content[: max_chars // 2])
    return normalize_ws(" ".join(parts))[:max_chars]


def chunk_views(content: str, *, chunk_chars: int, overlap_chars: int, max_chunks: int) -> list[str]:
    if chunk_chars <= 0:
        return []
    chunks: list[str] = []
    step = max(1, chunk_chars - overlap_chars)
    index = 0
    while index < len(content) and len(chunks) < max_chunks:
        chunk = normalize_ws(content[index : index + chunk_chars])
        if len(chunk) >= 40:
            chunks.append(chunk)
        index += step
    return chunks


def run(args: argparse.Namespace) -> dict[str, Any]:
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    rows_written = 0
    skipped = 0
    source_counts: dict[str, int] = {}

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        for doc_id, rel_path in sorted(uuid_index.items(), key=lambda item: item[1]):
            if args.max_docs is not None and rows_written >= args.max_docs:
                break
            path = args.sources_dir / str(rel_path)
            try:
                document = read_json(path)
                if not isinstance(document, dict):
                    skipped += 1
                    continue
                title, content = extract_document_content(document)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                skipped += 1
                continue
            src = source_type(str(rel_path))
            source_counts[src] = source_counts.get(src, 0) + 1
            row = {
                "doc_id": doc_id,
                "source_type": src,
                "path": rel_path,
                "title_view": normalize_ws(title)[: args.max_view_chars],
                "path_view": normalize_ws(str(rel_path).replace("/", " ").replace("-", " ")),
                "body_view": normalize_ws(content[: args.max_body_chars]),
                "source_metadata_view": metadata_view(document, max_chars=args.max_view_chars),
                "entity_view": entity_view(document, str(rel_path), title),
                "summary_view": summary_view(document, title, content, max_chars=args.max_view_chars),
                "chunk_views": chunk_views(
                    content,
                    chunk_chars=args.chunk_chars,
                    overlap_chars=args.chunk_overlap_chars,
                    max_chunks=args.max_chunks_per_doc,
                ),
            }
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")
            rows_written += 1

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.doc_views.v1",
        "uuid_index": str(args.uuid_index),
        "sources_dir": str(args.sources_dir),
        "output": str(args.output),
        "rows_written": rows_written,
        "skipped": skipped,
        "source_counts": dict(sorted(source_counts.items())),
        "max_docs": args.max_docs,
        "max_view_chars": args.max_view_chars,
        "max_body_chars": args.max_body_chars,
        "chunk_chars": args.chunk_chars,
        "chunk_overlap_chars": args.chunk_overlap_chars,
        "max_chunks_per_doc": args.max_chunks_per_doc,
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--max-docs", type=int)
    parser.add_argument("--max-view-chars", type=int, default=1200)
    parser.add_argument("--max-body-chars", type=int, default=2200)
    parser.add_argument("--chunk-chars", type=int, default=900)
    parser.add_argument("--chunk-overlap-chars", type=int, default=120)
    parser.add_argument("--max-chunks-per-doc", type=int, default=4)
    args = parser.parse_args()
    for name in ("max_view_chars", "max_body_chars", "chunk_chars", "max_chunks_per_doc"):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.chunk_overlap_chars < 0:
        parser.error("--chunk-overlap-chars must be non-negative")
    if args.max_docs is not None and args.max_docs <= 0:
        parser.error("--max-docs must be positive")
    return args


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
