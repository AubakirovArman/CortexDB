#!/usr/bin/env python3
"""Doc-view reranker for EnterpriseRAG-Bench retrieval candidates.

The reranker is deterministic and local-only. It uses multiple document views
and never calls an LLM. It is intended to exercise the production retrieval
ideas behind multi-view indexing, field-weighted scoring, chunk-to-document
aggregation, neighbor/thread awareness, and diversity-aware evidence selection.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
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
from hybrid_rerank_features import cosine, load_embedding_cache, normalize
from multi_index_candidate_generation import extract_document_content
from question_decomposition import covered_unit_ids, evidence_units, precise_anchors, tokens


QUERY_EXPANSIONS = {
    "blocked": ["blocker", "risk", "delayed", "dependency", "waiting"],
    "owner": ["assignee", "responsible", "lead", "dri", "reviewer"],
    "policy": ["standard", "requirement", "guideline", "procedure", "control"],
    "rollback": ["revert", "fallback", "restore", "recovery"],
    "latency": ["p95", "p99", "tail", "ms", "slow"],
    "cost": ["price", "billing", "invoice", "credits"],
    "security": ["compliance", "audit", "rbac", "auth", "kms"],
    "capacity": ["quota", "limit", "gpu", "pool", "burst"],
    "route": ["routing", "router", "policy", "traffic"],
    "support": ["ticket", "case", "escalation", "customer"],
    "deployment": ["rollout", "release", "upgrade", "canary"],
    "complete": ["all", "list", "every", "coverage"],
    "observability": ["telemetry", "metrics", "tracing", "trace", "logs", "jsonl"],
    "tracking": ["telemetry", "metrics", "trace", "tracing", "logging"],
    "invocation": ["function", "call", "function-call", "tool", "tool-calling"],
    "tool": ["function", "call", "function-call", "invocation"],
    "staged": ["rollout", "schedule", "phase", "phased", "canary"],
    "schedule": ["rollout", "timeline", "phase", "phased"],
    "fallback": ["failover", "demotion", "route", "routing", "backup"],
    "locked": ["pinned", "sticky", "fixed"],
    "model": ["variant", "version"],
    "scaler": ["autoscaler", "autoscale", "keda", "hpa"],
}

FIELD_WEIGHTS = {
    "title_view": 4.0,
    "path_view": 3.5,
    "source_metadata_view": 2.5,
    "entity_view": 3.2,
    "summary_view": 2.8,
    "body_view": 1.0,
    "chunk_views": 1.7,
}

DIVERSITY_TYPES = {
    "completeness",
    "conflicting_info",
    "project_related",
    "semantic",
    "high_level",
}


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


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        values[qid] = row
    return values


def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def tokenize_view(text: str) -> Counter[str]:
    return Counter(tokens(text))


def query_terms(question: dict[str, Any]) -> list[str]:
    text = str(question.get("question", ""))
    values = tokens(text)
    for anchor in precise_anchors(text):
        values.extend(tokens(anchor))
    for token in list(values):
        values.extend(QUERY_EXPANSIONS.get(token, []))
    return sorted(set(values), key=lambda item: (-len(item), item))


def query_context(question: dict[str, Any]) -> dict[str, Any]:
    question_text = str(question.get("question", ""))
    terms = query_terms(question)
    return {
        "question_id": str(question.get("question_id", "")),
        "question_text": question_text,
        "terms": terms,
        "counts": Counter(terms),
        "source_types": {str(item) for item in question.get("source_types", []) if str(item)},
        "anchors": precise_anchors(question_text),
        "units": evidence_units(question_text),
    }


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


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


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


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval_rows = rows_by_id(read_jsonl(args.retrieval_file), "retrieval")
    baseline_rows = (
        rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
        if args.baseline_retrieval_file
        else {}
    )
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    view_cache = ViewCache(uuid_index, args.sources_dir, args.doc_views_file)
    embeddings = load_embedding_cache(args.embedding_cache)

    output_rows: list[dict[str, Any]] = []
    diagnostics: dict[str, Any] = {}
    recall_values: list[float] = []
    per_type: dict[str, list[float]] = defaultdict(list)
    changed_rows = 0
    routed_rows = 0

    for qid, row in sorted(retrieval_rows.items()):
        question = questions.get(qid, row)
        context = query_context(question)
        baseline_row = baseline_rows.get(qid)
        baseline_ids = doc_ids(baseline_row)
        original_ids = baseline_ids[: args.limit] if baseline_row is not None else doc_ids(row)[: args.limit]
        if not route_enabled(question, args.route_question_types, args.route_source_types):
            output = dict(baseline_row if baseline_row is not None else row)
            output["document_ids"] = original_ids
            output["route"] = {
                "policy": args.policy_name,
                "enabled": False,
                "reason": "question_or_source_type_not_routed",
                "question_type": question.get("question_type"),
                "source_types": question.get("source_types", []),
            }
            output_rows.append(output)
            recall = recall_pct(question, original_ids)
            if recall is not None:
                recall_values.append(recall)
                per_type[str(question.get("question_type") or "unknown")].append(recall)
            continue
        candidate_ids = doc_ids(row)[: args.score_candidate_limit]
        if baseline_row is not None:
            candidate_ids = list(dict.fromkeys(baseline_ids + candidate_ids))
        scored: list[tuple[float, str, dict[str, Any], dict[str, Any]]] = []
        for rank, doc_id in enumerate(candidate_ids, 1):
            doc = view_cache.get(doc_id)
            features = score_doc(
                question=question,
                context=context,
                doc=doc,
                raw_rank=rank,
                embeddings=embeddings,
            )
            scored.append((float(features["score"]), doc_id, doc, features))
        scored.sort(key=lambda item: (-item[0], int(item[3]["rank"]), item[1]))
        selected, selection_diag = select_docs(
            question=question,
            scored_docs=[(doc_id, doc, features) for _score, doc_id, doc, features in scored],
            limit=args.limit,
            seed_count=args.seed_count,
        )
        selected = merge_with_baseline(
            baseline_ids=original_ids,
            reranked_ids=selected,
            limit=args.limit,
            protect_prefix=args.protect_baseline_prefix,
        )
        if str(question.get("question_type") or "") in args.raw_tail_question_types:
            selected = inject_raw_tail_candidates(
                selected=selected,
                candidate_ids=candidate_ids,
                limit=args.limit,
                tail_slots=args.raw_candidate_tail_slots,
                rank_limit=args.raw_candidate_tail_rank_limit,
            )
        output = dict(baseline_row if baseline_row is not None else row)
        if selected != original_ids:
            changed_rows += 1
        routed_rows += 1
        output["document_ids"] = selected
        output["route"] = {
            "policy": args.policy_name,
            "enabled": True,
            "limit": args.limit,
            "score_candidate_limit": args.score_candidate_limit,
            "seed_count": args.seed_count,
            "protect_baseline_prefix": args.protect_baseline_prefix,
            "raw_candidate_tail_slots": args.raw_candidate_tail_slots
            if str(question.get("question_type") or "") in args.raw_tail_question_types
            else 0,
            "raw_candidate_tail_rank_limit": args.raw_candidate_tail_rank_limit,
            "question_type": question.get("question_type"),
        }
        output_rows.append(output)
        recall = recall_pct(question, selected)
        if recall is not None:
            recall_values.append(recall)
            per_type[str(question.get("question_type") or "unknown")].append(recall)
        if args.diagnostics_top_k:
            diagnostics[qid] = {
                "recall_pct": recall,
                "selection": selection_diag,
                "top": [
                    {
                        "doc_id": doc_id,
                        "score": round(score, 4),
                        "path": uuid_index.get(doc_id, ""),
                        "features": features,
                    }
                    for score, doc_id, _doc, features in scored[: args.diagnostics_top_k]
                ],
                "selected": selected,
            }

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.doc_view_rerank.v1",
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "output": str(args.output),
        "policy_name": args.policy_name,
        "limit": args.limit,
        "score_candidate_limit": args.score_candidate_limit,
        "seed_count": args.seed_count,
        "protect_baseline_prefix": args.protect_baseline_prefix,
        "route_question_types": sorted(args.route_question_types),
        "route_source_types": sorted(args.route_source_types),
        "embedding_cache": str(args.embedding_cache) if args.embedding_cache else None,
        "embedding_vectors_loaded": len(embeddings),
        "doc_views_file": str(args.doc_views_file) if args.doc_views_file else None,
        "prebuilt_doc_views_loaded": len(view_cache.prebuilt),
        "changed_rows": changed_rows,
        "routed_rows": routed_rows,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "per_type_recall_pct": {
            key: round(sum(values) / len(values), 2)
            for key, values in sorted(per_type.items())
        },
        "diagnostics": diagnostics,
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--doc-views-file", type=Path)
    parser.add_argument("--embedding-cache", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="doc_view_rerank_v1")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--score-candidate-limit", type=int, default=140)
    parser.add_argument("--seed-count", type=int, default=4)
    parser.add_argument("--protect-baseline-prefix", type=int, default=8)
    parser.add_argument("--route-question-types", default="")
    parser.add_argument("--route-source-types", default="")
    parser.add_argument("--raw-tail-question-types", default="")
    parser.add_argument("--raw-candidate-tail-slots", type=int, default=0)
    parser.add_argument("--raw-candidate-tail-rank-limit", type=int, default=50)
    parser.add_argument("--diagnostics-top-k", type=int, default=5)
    args = parser.parse_args()
    for name in ("limit", "score_candidate_limit"):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.seed_count < 0:
        parser.error("--seed-count must be non-negative")
    if args.protect_baseline_prefix < 0:
        parser.error("--protect-baseline-prefix must be non-negative")
    if args.raw_candidate_tail_slots < 0:
        parser.error("--raw-candidate-tail-slots must be non-negative")
    if args.raw_candidate_tail_rank_limit <= 0:
        parser.error("--raw-candidate-tail-rank-limit must be positive")
    if args.diagnostics_top_k < 0:
        parser.error("--diagnostics-top-k must be non-negative")
    args.route_question_types = {
        value.strip()
        for value in args.route_question_types.split(",")
        if value.strip()
    }
    args.route_source_types = {
        value.strip()
        for value in args.route_source_types.split(",")
        if value.strip()
    }
    args.raw_tail_question_types = {
        value.strip()
        for value in args.raw_tail_question_types.split(",")
        if value.strip()
    }
    return args


def main() -> int:
    summary = run(parse_args())
    print(
        json.dumps(
            {
                "questions": summary["questions"],
                "average_recall_pct": summary["average_recall_pct"],
                "full_recall_questions": summary["full_recall_questions"],
                "changed_rows": summary["changed_rows"],
                "output": summary["output"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
