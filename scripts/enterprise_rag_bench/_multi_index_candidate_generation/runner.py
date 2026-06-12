from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from typing import Any

from .indexes import ContentPreviewIndex, PathIndex
from .io import read_json, read_jsonl, rows_by_id, source_type, write_json, write_jsonl
from .query import path_query_terms, query_phrases, query_terms, strong_uncapped_terms
from .scoring import add_rrf, doc_ids, recall_pct, route_settings

def run(args: argparse.Namespace) -> dict[str, Any]:
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    base_rows = rows_by_id(read_jsonl(args.base_retrieval_file), "base retrieval")
    extra_rows = [
        rows_by_id(read_jsonl(path), f"extra retrieval {path}")
        for path in args.extra_retrieval_file
    ]
    path_index = PathIndex(
        uuid_index,
        max_posting=args.max_posting,
        expand_ngrams=args.enable_path_ngrams,
    )
    all_query_terms = {
        term
        for question in questions.values()
        for term in query_terms(question)
        if len(term) > 1
    }
    all_uncapped_terms = {
        term
        for question in questions.values()
        for term in strong_uncapped_terms(question)
    }
    all_query_phrases = (
        {
            phrase
            for question in questions.values()
            for phrase in query_phrases(question)
            if len(phrase) > 3
        }
        if args.phrase_candidate_limit > 0 and args.phrase_boost_limit > 0
        else set()
    )
    content_index = (
        ContentPreviewIndex(
            uuid_index,
            args.sources_dir,
            target_terms=all_query_terms,
            target_phrases=all_query_phrases,
            uncapped_terms=all_uncapped_terms,
            max_posting=args.max_posting,
            phrase_max_posting=args.phrase_max_posting,
            preview_chars=args.content_preview_chars,
            include_source_links=args.enable_source_link_neighbors,
        )
        if args.sources_dir
        else None
    )

    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    source_counts: Counter[str] = Counter()
    route_counts: Counter[str] = Counter()
    diagnostics: dict[str, Any] = {}

    for qid, question in questions.items():
        question_type = str(question.get("question_type", ""))
        settings = route_settings(question_type, args)
        route_counts[str(settings["policy"]) + ":" + (question_type or "unknown")] += 1
        source_types = {str(item) for item in question.get("source_types", []) if str(item)}
        terms = query_terms(question)
        term_set = set(terms)
        phrases = query_phrases(question)
        phrase_set = set(phrases)
        path_terms = path_query_terms(question) if args.path_terms_mode == "entity" else terms
        path_term_set = set(path_terms)
        scores: dict[str, float] = {}
        score_sources: dict[str, set[str]] = defaultdict(set)

        base_doc_ids = doc_ids(base_rows.get(qid))
        add_rrf(scores, base_doc_ids[: args.base_limit], weight=args.weight_base_rrf, k=args.rrf_k)
        for doc_id in base_doc_ids[: args.base_limit]:
            score_sources[doc_id].add("base")

        for extra_index, rows in enumerate(extra_rows, 1):
            ids = doc_ids(rows.get(qid))[: args.extra_limit]
            add_rrf(scores, ids, weight=args.weight_extra_rrf, k=args.rrf_k)
            for doc_id in ids:
                score_sources[doc_id].add(f"extra_{extra_index}")

        path_docs = path_index.candidate_ids_for_terms(
            path_terms,
            source_types,
            max_docs=args.path_candidate_limit,
        )
        for doc_id in path_docs:
            if args.path_existing_only and doc_id not in scores:
                continue
            score = path_index.path_score(path_term_set, source_types, doc_id)
            if score <= 0.0:
                continue
            scores[doc_id] = scores.get(doc_id, 0.0) + score * float(settings["path_weight"])
            score_sources[doc_id].add("path")

        if content_index:
            content_existing_only = bool(settings["content_existing_only"])
            content_docs = content_index.candidate_ids_for_terms(
                terms,
                source_types,
                max_docs=args.content_candidate_limit,
            )
            scored_content: list[tuple[float, str]] = []
            for doc_id in content_docs:
                if content_existing_only and doc_id not in scores:
                    continue
                score = content_index.content_score(term_set, source_types, doc_id)
                if score < args.content_score_threshold:
                    continue
                scored_content.append((score, doc_id))
            scored_content.sort(key=lambda item: (-item[0], path_index.uuid_index.get(item[1], ""), item[1]))
            for score, doc_id in scored_content[: int(settings["content_boost_limit"])]:
                scores[doc_id] = scores.get(doc_id, 0.0) + score * float(settings["content_weight"])
                score_sources[doc_id].add("content_preview")

            if args.phrase_candidate_limit > 0 and args.phrase_boost_limit > 0 and phrases:
                phrase_docs = content_index.candidate_ids_for_phrases(
                    phrases,
                    source_types,
                    max_docs=args.phrase_candidate_limit,
                )
                scored_phrases: list[tuple[float, str]] = []
                for doc_id in phrase_docs:
                    score = content_index.phrase_score(phrase_set, source_types, doc_id)
                    if score <= 0.0:
                        continue
                    scored_phrases.append((score, doc_id))
                scored_phrases.sort(key=lambda item: (-item[0], path_index.uuid_index.get(item[1], ""), item[1]))
                for score, doc_id in scored_phrases[: args.phrase_boost_limit]:
                    scores[doc_id] = scores.get(doc_id, 0.0) + score * args.weight_phrase
                    score_sources[doc_id].add("source_phrase")

            if args.neighbor_expansion_limit > 0 and scores:
                seed_ids = sorted(
                    scores,
                    key=lambda doc_id: (
                        -scores[doc_id],
                        path_index.uuid_index.get(doc_id, ""),
                        doc_id,
                    ),
                )[: args.neighbor_seed_limit]
                for score, doc_id in content_index.neighbor_scores(
                    seed_ids,
                    source_types,
                    max_docs=args.neighbor_expansion_limit,
                    max_per_seed=args.neighbor_max_per_seed,
                    max_posting=args.neighbor_max_posting,
                ):
                    scores[doc_id] = scores.get(doc_id, 0.0) + score * args.weight_neighbor
                    score_sources[doc_id].add("neighbor")

        if source_types and args.source_match_boost != 0.0:
            for doc_id in list(scores):
                doc_source = source_type(path_index.uuid_index.get(doc_id, ""))
                if doc_source in source_types:
                    scores[doc_id] += args.source_match_boost
                    score_sources[doc_id].add("source_type_boost")

        reranked = sorted(
            scores,
            key=lambda doc_id: (
                -scores[doc_id],
                -len(score_sources[doc_id]),
                path_index.uuid_index.get(doc_id, ""),
                doc_id,
            ),
        )
        selected = reranked[: args.top_k]
        recall = recall_pct(question, selected)
        if recall is not None:
            recall_values.append(recall)
        for doc_id in selected:
            for source in score_sources.get(doc_id, {"unknown"}):
                source_counts[source] += 1
        output_rows.append(
            {
                "answer": "",
                "document_ids": selected,
                "question": question.get("question", ""),
                "question_id": qid,
                "question_type": question.get("question_type"),
                "route": {
                    "policy": settings["policy"],
                    "top_k": args.top_k,
                    "content_boost_limit": int(settings["content_boost_limit"]),
                    "content_existing_only": bool(settings["content_existing_only"]),
                    "content_weight": float(settings["content_weight"]),
                    "path_weight": float(settings["path_weight"]),
                    "source_types": sorted(source_types),
                },
            }
        )
        if args.diagnostics_top_k > 0:
            diagnostics[qid] = {
                "recall_pct": recall,
                "terms": terms[:24],
                "path_terms": path_terms[:24],
                "route": settings,
                "candidate_sources": [
                    {
                        "doc_id": doc_id,
                        "score": round(scores[doc_id], 4),
                        "sources": sorted(score_sources[doc_id]),
                        "path": path_index.uuid_index.get(doc_id, ""),
                    }
                    for doc_id in selected[: args.diagnostics_top_k]
                ],
            }

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.multi_index_candidates.v1",
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "base_retrieval_file": str(args.base_retrieval_file),
        "extra_retrieval_files": [str(path) for path in args.extra_retrieval_file],
        "uuid_index": str(args.uuid_index),
        "output": str(args.output),
        "top_k": args.top_k,
        "base_limit": args.base_limit,
        "extra_limit": args.extra_limit,
        "path_candidate_limit": args.path_candidate_limit,
        "path_existing_only": args.path_existing_only,
        "path_terms_mode": args.path_terms_mode,
        "enable_path_ngrams": args.enable_path_ngrams,
        "content_candidate_limit": args.content_candidate_limit,
        "content_boost_limit": args.content_boost_limit,
        "content_preview_chars": args.content_preview_chars,
        "content_score_threshold": args.content_score_threshold,
        "phrase_candidate_limit": args.phrase_candidate_limit,
        "phrase_boost_limit": args.phrase_boost_limit,
        "phrase_max_posting": args.phrase_max_posting,
        "weight_phrase": args.weight_phrase,
        "neighbor_expansion_limit": args.neighbor_expansion_limit,
        "neighbor_seed_limit": args.neighbor_seed_limit,
        "neighbor_max_per_seed": args.neighbor_max_per_seed,
        "neighbor_max_posting": args.neighbor_max_posting,
        "weight_neighbor": args.weight_neighbor,
        "enable_source_link_neighbors": args.enable_source_link_neighbors,
        "max_posting": args.max_posting,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "source_counts": dict(sorted(source_counts.items())),
        "source_match_boost": args.source_match_boost,
        "route_counts": dict(sorted(route_counts.items())),
        "content_index": content_index.report() if content_index else None,
        "diagnostics": diagnostics,
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report

