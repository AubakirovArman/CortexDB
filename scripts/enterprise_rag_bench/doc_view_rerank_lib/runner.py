from __future__ import annotations

import argparse
from collections import defaultdict
from typing import Any

from hybrid_rerank_features import load_embedding_cache

from .files import doc_ids, read_json, read_jsonl, rows_by_id, write_json, write_jsonl
from .metrics import recall_pct
from .query import query_context
from .scoring import score_doc
from .selection import inject_raw_tail_candidates, merge_with_baseline, route_enabled, select_docs
from .views import ViewCache

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
