from __future__ import annotations

import json
import math
import time
from pathlib import Path
from typing import Any

from . import logging as vector_logging
from .files import read_json, read_jsonl


def quantize_unit_i16(vector: list[float], scale: int) -> list[int]:
    norm = math.sqrt(sum(item * item for item in vector))
    if norm == 0:
        return [0 for _ in vector]
    return [
        max(-32768, min(32767, int(round(item / norm * scale)))) for item in vector
    ]


def extract_document_content(doc: dict[str, Any]) -> tuple[str, str]:
    title_field = doc.get("title_field_name")
    content_fields = doc.get("content_field_names")
    title = str(doc.get(title_field, "")) if isinstance(title_field, str) else ""
    if not isinstance(content_fields, list) or not content_fields:
        return title, json.dumps(doc, ensure_ascii=False)
    parts: list[str] = []
    for field in content_fields:
        if not isinstance(field, str) or field not in doc:
            continue
        value = doc[field]
        if isinstance(value, list):
            value = "\n".join(str(item) for item in value)
        elif isinstance(value, dict):
            value = json.dumps(value, ensure_ascii=False)
        parts.append(f"{field}:\n{value}" if len(content_fields) > 1 else str(value))
    return title, "\n\n".join(parts)


def question_texts(path: Path, limit: int | None) -> dict[str, str]:
    rows = read_jsonl(path)
    if limit is not None:
        rows = rows[:limit]
    texts = {}
    for index, row in enumerate(rows, 1):
        qid = str(row.get("question_id") or "")
        question = str(row.get("question") or "")
        if not qid or not question.strip():
            raise ValueError(f"{path}:{index}: missing question_id or question")
        texts[qid] = question
    return texts


def document_texts(
    uuid_index_path: Path,
    sources_dir: Path,
    limit: int | None,
    max_chars: int,
) -> dict[str, str]:
    uuid_index = read_json(uuid_index_path)
    if not isinstance(uuid_index, dict):
        raise ValueError(f"{uuid_index_path}: expected JSON object")
    texts = {}
    started = time.perf_counter()
    total = min(len(uuid_index), limit) if limit is not None else len(uuid_index)
    vector_logging.log(f"load document texts start total={total} max_chars={max_chars}")
    vector_logging.LOGGER.status(
        stage="load_document_texts",
        state="running",
        scanned_documents=0,
        total_documents=total,
        kept_documents=0,
    )
    scanned = 0
    for index, (doc_id, rel_path) in enumerate(uuid_index.items(), 1):
        if limit is not None and len(texts) >= limit:
            break
        scanned = index
        if not isinstance(rel_path, str):
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        text = f"{title}\n\n{content}"[:max_chars].strip()
        if text:
            texts[str(doc_id)] = text
        if index % 50_000 == 0:
            elapsed = max(0.0, time.perf_counter() - started)
            rate = index / elapsed if elapsed > 0 else 0.0
            remaining = max(0, total - index)
            eta = remaining / rate if rate > 0.0 else None
            eta_label = f"{eta:.1f}s" if eta is not None else "unknown"
            vector_logging.log(
                "loaded document texts "
                f"scanned={index}/{total} kept={len(texts)} "
                f"rate={rate:.2f}/s eta={eta_label}"
            )
            vector_logging.LOGGER.status(
                stage="load_document_texts",
                state="running",
                scanned_documents=index,
                total_documents=total,
                kept_documents=len(texts),
                rate_per_second=round(rate, 4),
                eta_seconds=round(eta, 1) if eta is not None else None,
            )
    elapsed = max(0.0, time.perf_counter() - started)
    rate = scanned / elapsed if elapsed > 0.0 else 0.0
    vector_logging.log(
        f"load document texts done scanned={scanned} kept={len(texts)} rate={rate:.2f}/s"
    )
    vector_logging.LOGGER.status(
        stage="load_document_texts",
        state="done",
        scanned_documents=scanned,
        total_documents=total,
        kept_documents=len(texts),
        rate_per_second=round(rate, 4),
    )
    return texts


def write_vectors(
    path: Path,
    id_field: str,
    vectors: dict[str, list[float]],
    *,
    scale: int,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for item_id, vector in vectors.items():
            row = {
                id_field: item_id,
                "vector": quantize_unit_i16(vector, scale),
            }
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")
