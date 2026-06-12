from __future__ import annotations

import math
from collections import Counter
from typing import Any

from hybrid_rerank_features import cosine
from question_decomposition import covered_unit_ids

from .constants import FIELD_WEIGHTS

def field_score(query_counts: Counter[str], counts: Counter[str]) -> float:
    if not counts:
        return 0.0
    score = 0.0
    for token, q_count in query_counts.items():
        if token not in counts:
            continue
        score += min(counts[token], 3) * (1.0 + math.log1p(q_count))
    return score


def chunk_score(query_counts: Counter[str], chunks: list[Counter[str]]) -> tuple[float, int]:
    best = 0.0
    best_index = -1
    for index, chunk in enumerate(chunks):
        score = field_score(query_counts, chunk)
        if score > best:
            best = score
            best_index = index
    return best, best_index


def score_doc(
    *,
    question: dict[str, Any],
    context: dict[str, Any],
    doc: dict[str, Any],
    raw_rank: int,
    embeddings: dict[str, list[float]],
) -> dict[str, Any]:
    qid = str(context["question_id"])
    q_counts = context["counts"]
    source_types = context["source_types"]
    field_scores: dict[str, float] = {}
    for field in ("title_view", "path_view", "source_metadata_view", "entity_view", "summary_view", "body_view"):
        field_scores[field] = field_score(q_counts, doc["view_counts"].get(field, Counter()))
    best_chunk_score, best_chunk_index = chunk_score(q_counts, doc.get("chunk_counts", []))
    field_scores["chunk_views"] = best_chunk_score

    anchors = context["anchors"]
    anchor_hits = sum(1 for anchor in anchors if anchor in doc["normalized"])
    source_match = 1.0 if source_types and doc.get("source_type") in source_types else 0.0
    units = context["units"]
    covered_units = covered_unit_ids(units, doc["normalized"], doc["token_set"])
    evidence_coverage = len(covered_units) / len(units) if units else 0.0
    embedding = cosine(embeddings.get(f"q:{qid}"), embeddings.get(f"d:{doc['doc_id']}"))
    weighted_fields = sum(field_scores[field] * FIELD_WEIGHTS[field] for field in FIELD_WEIGHTS)
    rank_prior = 1.0 / max(raw_rank, 1)
    top20_prior = max(0.0, (21.0 - raw_rank) / 20.0)
    score = (
        weighted_fields
        + anchor_hits * 22.0
        + source_match * 18.0
        + len(covered_units) * 16.0
        + evidence_coverage * 28.0
        + embedding * 65.0
        + rank_prior * 55.0
        + top20_prior * 34.0
    )
    return {
        "score": score,
        "field_scores": field_scores,
        "best_chunk_index": best_chunk_index,
        "anchor_hits": anchor_hits,
        "source_match": source_match,
        "covered_evidence_units": covered_units,
        "evidence_coverage": evidence_coverage,
        "embedding": embedding,
        "raw_rank": 1.0 / max(raw_rank, 1),
        "rank": raw_rank,
        "neighbor_keys": doc.get("neighbor_keys", []),
    }
