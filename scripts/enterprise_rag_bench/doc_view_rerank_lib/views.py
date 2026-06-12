from __future__ import annotations

from collections import Counter
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
from hybrid_rerank_features import normalize
from multi_index_candidate_generation import extract_document_content
from question_decomposition import tokens

from .files import read_json, read_jsonl

def tokenize_view(text: str) -> Counter[str]:
    return Counter(tokens(text))

def neighbor_keys(document: dict[str, Any], rel_path: str) -> set[str]:
    keys: set[str] = set()
    source = source_type(rel_path)
    keys.add(f"source:{source}")
    path_parts = rel_path.split("/")
    if len(path_parts) >= 3:
        keys.add("dir:" + "/".join(path_parts[:3]))
    for field in (
        "thread_id",
        "thread_ts",
        "repo",
        "project",
        "company_id",
        "company_name",
        "customer_company",
        "related_account",
        "crm_deal_id",
        "crm_account_id",
        "key",
        "channel",
    ):
        value = document.get(field)
        if isinstance(value, str) and value:
            keys.add(f"{field}:{normalize(value)}")
    return keys


class ViewCache:
    def __init__(self, uuid_index: dict[str, str], sources_dir: Path, doc_views_file: Path | None) -> None:
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.prebuilt = self._load_prebuilt(doc_views_file)
        self.values: dict[str, dict[str, Any]] = {}

    @staticmethod
    def _load_prebuilt(path: Path | None) -> dict[str, dict[str, Any]]:
        if path is None or not path.exists():
            return {}
        values: dict[str, dict[str, Any]] = {}
        for row in read_jsonl(path):
            doc_id = row.get("doc_id")
            if isinstance(doc_id, str) and doc_id:
                values[doc_id] = row
        return values

    def get(self, doc_id: str) -> dict[str, Any]:
        if doc_id in self.values:
            return self.values[doc_id]
        if doc_id in self.prebuilt:
            value = self._finalize(dict(self.prebuilt[doc_id]))
            self.values[doc_id] = value
            return value
        rel_path = self.uuid_index.get(doc_id, "")
        path = self.sources_dir / rel_path if rel_path else None
        document: dict[str, Any] = {}
        title = ""
        content = ""
        if path is not None:
            try:
                loaded = read_json(path)
                if isinstance(loaded, dict):
                    document = loaded
                    title, content = extract_document_content(document)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                pass
        value = {
            "doc_id": doc_id,
            "path": rel_path,
            "source_type": source_type(rel_path),
            "title_view": normalize_ws(title),
            "path_view": normalize_ws(rel_path.replace("/", " ").replace("-", " ").replace("_", " ")),
            "body_view": normalize_ws(content[:2200]),
            "source_metadata_view": metadata_view(document, max_chars=1200),
            "entity_view": entity_view(document, rel_path, title),
            "summary_view": summary_view(document, title, content, max_chars=1200),
            "chunk_views": chunk_views(content, chunk_chars=900, overlap_chars=120, max_chunks=4),
            "neighbor_keys": sorted(neighbor_keys(document, rel_path)),
        }
        count_fields = (
            "title_view",
            "path_view",
            "source_metadata_view",
            "entity_view",
            "summary_view",
            "body_view",
        )
        value = self._finalize(value)
        self.values[doc_id] = value
        return value

    def _finalize(self, value: dict[str, Any]) -> dict[str, Any]:
        rel_path = str(value.get("path") or self.uuid_index.get(str(value.get("doc_id", "")), ""))
        value.setdefault("path", rel_path)
        value.setdefault("source_type", source_type(rel_path))
        value.setdefault("title_view", "")
        value.setdefault("path_view", normalize_ws(rel_path.replace("/", " ").replace("-", " ").replace("_", " ")))
        value.setdefault("source_metadata_view", "")
        value.setdefault("entity_view", "")
        value.setdefault("summary_view", "")
        value.setdefault("body_view", "")
        value.setdefault("chunk_views", [])
        if "neighbor_keys" not in value:
            value["neighbor_keys"] = sorted(neighbor_keys({}, rel_path))
        count_fields = (
            "title_view",
            "path_view",
            "source_metadata_view",
            "entity_view",
            "summary_view",
            "body_view",
        )
        value["view_counts"] = {
            field: tokenize_view(str(value.get(field, ""))) for field in count_fields
        }
        value["chunk_counts"] = [
            tokenize_view(str(chunk)) for chunk in value.get("chunk_views", [])
        ]
        value["all_text"] = "\n".join(
            [
                str(value["path_view"]),
                str(value["title_view"]),
                str(value["source_metadata_view"]),
                str(value["entity_view"]),
                str(value["summary_view"]),
                str(value["body_view"]),
                "\n".join(str(item) for item in value["chunk_views"]),
            ]
        )
        value["normalized"] = normalize(str(value["all_text"]))
        value["token_set"] = set(tokens(str(value["all_text"])))
        return value
