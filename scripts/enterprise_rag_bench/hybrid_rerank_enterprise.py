#!/usr/bin/env python3
"""Hybrid evidence-aware reranker for EnterpriseRAG-Bench retrieval rows."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from hybrid_rerank_features import (
    DocumentCache,
    load_embedding_cache,
    query_idf,
    read_json,
    read_jsonl,
    score_doc,
    tokens,
)


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def final_limit(question: dict[str, Any], args: argparse.Namespace) -> int:
    question_type = str(question.get("question_type") or "")
    if question_type == "basic":
        return args.basic_top_k
    if question_type in {"completeness", "conflicting_info", "project_related"}:
        return args.multi_doc_top_k
    return args.top_k


def recall_pct(question: dict[str, Any], doc_ids: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", [])}
    if not expected:
        return None
    return round(len(expected & set(doc_ids)) / len(expected) * 100.0, 2)


def default_weights(args: argparse.Namespace) -> dict[str, float]:
    return {
        "weighted_overlap": args.weight_weighted_overlap,
        "repeat_overlap": args.weight_repeat_overlap,
        "coverage": args.weight_coverage,
        "anchor": args.weight_anchor,
        "anchor_ratio": args.weight_anchor_ratio,
        "phrase": args.weight_phrase,
        "title": args.weight_title,
        "path": args.weight_path,
        "digest": args.weight_digest,
        "source": args.weight_source,
        "embedding": args.weight_embedding,
        "raw_rank": args.weight_raw_rank,
        "top20_rank": args.weight_top20_rank,
    }


def rerank_row(
    *,
    row: dict[str, Any],
    question: dict[str, Any],
    doc_cache: DocumentCache,
    embeddings: dict[str, list[float]],
    weights: dict[str, float],
    args: argparse.Namespace,
) -> tuple[dict[str, Any], dict[str, Any], float | None]:
    docs = [doc_cache.get(str(doc_id)) for doc_id in row.get("document_ids", [])]
    q_tokens = [token for token in tokens(str(question.get("question", ""))) if token]
    idf = query_idf(q_tokens, docs)
    scored = []
    for rank, doc in enumerate(docs, 1):
        features = score_doc(
            question=question,
            doc=doc,
            rank=rank,
            embeddings=embeddings,
            idf=idf,
            weights=weights,
        )
        scored.append((features["score"], rank, str(doc["doc_id"]), features))
    scored.sort(key=lambda item: (-item[0], item[1], item[2]))
    limit = final_limit(question, args)
    selected = [doc_id for _score, _rank, doc_id, _features in scored[:limit]]
    output_row = {
        **row,
        "document_ids": selected,
        "route": {"policy": "hybrid_enterprise_v3", "selected_count": limit},
    }
    recall = recall_pct(question, selected)
    diagnostic = {
        "selected_count": len(selected),
        "recall_pct": recall,
        "top_features": [
            {"doc_id": doc_id, **features}
            for _score, _rank, doc_id, features in scored[: min(args.diagnostic_top_k, len(scored))]
        ],
    }
    return output_row, diagnostic, recall


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = {str(row["question_id"]): row for row in read_jsonl(args.questions_file)}
    rows = read_jsonl(args.retrieval_file)
    if args.limit is not None:
        rows = rows[: args.limit]
    doc_cache = DocumentCache(read_json(args.uuid_index), args.sources_dir)
    embeddings = load_embedding_cache(args.embedding_cache)
    weights = default_weights(args)

    output_rows: list[dict[str, Any]] = []
    diagnostics: dict[str, Any] = {}
    recall_values: list[float] = []
    for row in rows:
        qid = str(row.get("question_id"))
        question = questions.get(qid, row)
        output_row, diagnostic, recall = rerank_row(
            row=row,
            question=question,
            doc_cache=doc_cache,
            embeddings=embeddings,
            weights=weights,
            args=args,
        )
        output_rows.append(output_row)
        diagnostics[qid] = diagnostic
        if recall is not None:
            recall_values.append(recall)

    write_jsonl(args.output, output_rows)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.hybrid_rerank_report.v3",
        "questions": len(output_rows),
        "input": str(args.retrieval_file),
        "output": str(args.output),
        "report": str(args.report),
        "weights": weights,
        "top_k": args.top_k,
        "basic_top_k": args.basic_top_k,
        "multi_doc_top_k": args.multi_doc_top_k,
        "embedding_cache": str(args.embedding_cache) if args.embedding_cache else None,
        "embedding_vectors_loaded": len(embeddings),
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "diagnostics": diagnostics,
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--embedding-cache", type=Path)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--top-k", type=int, default=5)
    parser.add_argument("--basic-top-k", type=int, default=5)
    parser.add_argument("--multi-doc-top-k", type=int, default=8)
    parser.add_argument("--diagnostic-top-k", type=int, default=5)
    parser.add_argument("--weight-weighted-overlap", type=float, default=7.0)
    parser.add_argument("--weight-repeat-overlap", type=float, default=0.8)
    parser.add_argument("--weight-coverage", type=float, default=18.0)
    parser.add_argument("--weight-anchor", type=float, default=18.0)
    parser.add_argument("--weight-anchor-ratio", type=float, default=35.0)
    parser.add_argument("--weight-phrase", type=float, default=2.5)
    parser.add_argument("--weight-title", type=float, default=10.0)
    parser.add_argument("--weight-path", type=float, default=4.0)
    parser.add_argument("--weight-digest", type=float, default=0.8)
    parser.add_argument("--weight-source", type=float, default=14.0)
    parser.add_argument("--weight-embedding", type=float, default=60.0)
    parser.add_argument("--weight-raw-rank", type=float, default=0.0)
    parser.add_argument("--weight-top20-rank", type=float, default=25.0)
    args = parser.parse_args()
    if args.top_k <= 0 or args.basic_top_k <= 0 or args.multi_doc_top_k <= 0:
        parser.error("top-k values must be positive")
    return args


def main() -> int:
    report = run(parse_args())
    summary = {
        "questions": report["questions"],
        "average_recall_pct": report["average_recall_pct"],
        "full_recall_questions": report["full_recall_questions"],
        "output": report["output"],
        "report": report.get("report"),
    }
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
