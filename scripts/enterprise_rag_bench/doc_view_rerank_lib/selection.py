from __future__ import annotations

from typing import Any

from .constants import DIVERSITY_TYPES

def cluster_key(doc: dict[str, Any]) -> str:
    keys = [str(item) for item in doc.get("neighbor_keys", [])]
    for prefix in ("thread_id:", "thread_ts:", "repo:", "project:", "company_id:", "channel:", "dir:"):
        for key in keys:
            if key.startswith(prefix):
                return key
    return f"doc:{doc['doc_id']}"


def select_docs(
    *,
    question: dict[str, Any],
    scored_docs: list[tuple[str, dict[str, Any], dict[str, Any]]],
    limit: int,
    seed_count: int,
) -> tuple[list[str], dict[str, Any]]:
    qtype = str(question.get("question_type") or "")
    if qtype not in DIVERSITY_TYPES:
        return [doc_id for doc_id, _doc, _features in scored_docs[:limit]], {"enabled": False}

    pool = scored_docs[: max(limit, 80)]
    selected: list[tuple[str, dict[str, Any], dict[str, Any]]] = []
    selected_ids: set[str] = set()
    covered_units: set[str] = set()
    used_clusters: set[str] = set()

    for item in pool[: min(seed_count, limit)]:
        doc_id, doc, features = item
        selected.append(item)
        selected_ids.add(doc_id)
        used_clusters.add(cluster_key(doc))
        covered_units.update(str(unit) for unit in features.get("covered_evidence_units", []))

    while len(selected) < limit and len(selected) < len(pool):
        best: tuple[float, tuple[str, dict[str, Any], dict[str, Any]]] | None = None
        for item in pool:
            doc_id, doc, features = item
            if doc_id in selected_ids:
                continue
            item_units = {str(unit) for unit in features.get("covered_evidence_units", [])}
            new_units = item_units - covered_units
            doc_cluster = cluster_key(doc)
            diversity_bonus = 10.0 if doc_cluster not in used_clusters else -14.0
            neighbor_bonus = 0.0
            if any(key in used_clusters for key in doc.get("neighbor_keys", [])):
                neighbor_bonus = 6.0
            score = (
                float(features["score"])
                + len(new_units) * 28.0
                + float(features.get("evidence_coverage", 0.0)) * 18.0
                + diversity_bonus
                + neighbor_bonus
            )
            if best is None or score > best[0]:
                best = (score, item)
        if best is None:
            break
        selected_item = best[1]
        doc_id, doc, features = selected_item
        selected.append(selected_item)
        selected_ids.add(doc_id)
        used_clusters.add(cluster_key(doc))
        covered_units.update(str(unit) for unit in features.get("covered_evidence_units", []))
    return [doc_id for doc_id, _doc, _features in selected[:limit]], {
        "enabled": True,
        "covered_units": sorted(covered_units),
        "clusters": sorted(used_clusters),
    }


def merge_with_baseline(
    *,
    baseline_ids: list[str],
    reranked_ids: list[str],
    limit: int,
    protect_prefix: int,
) -> list[str]:
    selected: list[str] = []
    seen: set[str] = set()
    for doc_id in baseline_ids[: max(0, min(protect_prefix, limit))]:
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)
    for doc_id in reranked_ids:
        if len(selected) >= limit:
            break
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)
    for doc_id in baseline_ids:
        if len(selected) >= limit:
            break
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)
    return selected[:limit]


def inject_raw_tail_candidates(
    *,
    selected: list[str],
    candidate_ids: list[str],
    limit: int,
    tail_slots: int,
    rank_limit: int,
) -> list[str]:
    if tail_slots <= 0:
        return selected[:limit]
    protected = selected[: max(0, limit - tail_slots)]
    seen = set(protected)
    tail: list[str] = []
    for doc_id in candidate_ids[:rank_limit]:
        if doc_id in seen:
            continue
        tail.append(doc_id)
        seen.add(doc_id)
        if len(tail) >= tail_slots:
            break
    for doc_id in selected:
        if len(protected) + len(tail) >= limit:
            break
        if doc_id in seen:
            continue
        tail.append(doc_id)
        seen.add(doc_id)
    return (protected + tail)[:limit]


def route_enabled(
    question: dict[str, Any],
    route_types: set[str],
    route_source_types: set[str],
) -> bool:
    if not route_types:
        type_enabled = True
    else:
        type_enabled = str(question.get("question_type") or "") in route_types
    if not type_enabled:
        return False
    if not route_source_types:
        return True
    question_sources = {str(item) for item in question.get("source_types", []) if str(item)}
    return bool(question_sources & route_source_types)
