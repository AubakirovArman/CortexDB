"""Resume-state and validation helpers for corpus embeddings."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


def vector_is_valid(vector, expected_dimension: int | None) -> bool:
    if not isinstance(vector, list) or not vector:
        return False
    if expected_dimension is not None and len(vector) != expected_dimension:
        return False
    return True


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def load_done_ids(
    output: Path,
    expected_dimension: int | None = None,
    *,
    expected_model: str | None = None,
    expected_text_hashes: dict[str, str] | None = None,
) -> set[str]:
    """doc_ids already embedded (resume support)."""
    done: set[str] = set()
    if not output.exists():
        return done
    with output.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            doc_id = row.get("doc_id")
            vector = row.get("vector")
            if not isinstance(doc_id, str) or not doc_id:
                continue
            if not vector_is_valid(vector, expected_dimension):
                continue
            if expected_model is not None and row.get("model") != expected_model:
                continue
            expected_hash = expected_text_hashes.get(doc_id) if expected_text_hashes else None
            if expected_hash is not None and row.get("text_hash") != expected_hash:
                continue
            done.add(doc_id)
    return done


def read_only_ids(path: Path, uuid_index: dict) -> list[str]:
    requested: list[str] = []
    seen: set[str] = set()
    unknown: list[str] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        doc_id = raw.strip()
        if not doc_id or doc_id in seen:
            continue
        seen.add(doc_id)
        if doc_id not in uuid_index:
            unknown.append(doc_id)
            continue
        requested.append(doc_id)
    if unknown:
        sample = ", ".join(unknown[:10])
        raise ValueError(f"{path} contains {len(unknown)} unknown doc_ids; sample: {sample}")
    return requested


def chunks(items: list, size: int):
    for start in range(0, len(items), size):
        yield items[start : start + size]

