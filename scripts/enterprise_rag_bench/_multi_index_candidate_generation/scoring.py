from __future__ import annotations

import argparse
from typing import Any

from .query import QUERY_TYPE_ROUTE_PRESETS

def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def add_rrf(scores: dict[str, float], doc_ids_: list[str], *, weight: float, k: int) -> None:
    for rank, doc_id in enumerate(doc_ids_, 1):
        scores[doc_id] = scores.get(doc_id, 0.0) + weight / (k + rank)


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def route_settings(question_type: str, args: argparse.Namespace) -> dict[str, Any]:
    settings = {
        "policy": "multi_index_candidates_v4_content_safe",
        "content_boost_limit": args.content_boost_limit,
        "content_existing_only": question_type in args.content_existing_only_question_type,
        "content_weight": args.weight_content,
        "path_weight": args.weight_path,
    }
    if args.enable_query_type_router:
        preset = QUERY_TYPE_ROUTE_PRESETS.get(question_type, {})
        settings.update(preset)
        settings["policy"] = "multi_index_candidates_query_type_router_v1"
    return settings

