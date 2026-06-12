"""Expected text-hash manifest helpers for embedding staleness checks."""

from __future__ import annotations

import json
from pathlib import Path

from embed_corpus_lib.constants import EMBEDDING_MANIFEST_SCHEMA
from embed_corpus_lib.state import text_sha256
from rerank_with_embeddings import load_doc_text


def load_texts_and_hashes(
    ids: list[str],
    uuid_index: dict,
    sources_dir: Path,
    max_chars: int,
) -> tuple[dict[str, str], dict[str, str]]:
    texts: dict[str, str] = {}
    hashes: dict[str, str] = {}
    for doc_id in ids:
        text = load_doc_text(doc_id, uuid_index, sources_dir, max_chars)
        texts[doc_id] = text
        hashes[doc_id] = text_sha256(text)
    return texts, hashes


def write_expected_manifest(
    path: Path,
    ids: list[str],
    hashes: dict[str, str],
    model: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for doc_id in ids:
            handle.write(
                json.dumps(
                    {
                        "schema_version": EMBEDDING_MANIFEST_SCHEMA,
                        "doc_id": doc_id,
                        "model": model,
                        "text_hash": hashes.get(doc_id),
                    },
                    ensure_ascii=True,
                    sort_keys=True,
                )
                + "\n"
            )

