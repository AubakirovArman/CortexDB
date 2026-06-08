#!/usr/bin/env python3
"""Evaluate high-level EnterpriseRAG questions with fact coverage, not doc recall."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from evaluate_evidence_pack import (
    DocumentCache,
    fact_coverage,
    packed_text,
    read_json,
    read_jsonl,
    rows_by_id,
    write_json,
    write_jsonl,
)


def doc_ids(row: dict[str, Any] | None, limit: int) -> list[str]:
    if row is None:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)][:limit]


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval = rows_by_id(read_jsonl(args.retrieval_file), "retrieval")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    cache = DocumentCache(uuid_index, args.sources_dir)

    details: list[dict[str, Any]] = []
    token_coverages: list[float] = []
    full_coverages: list[float] = []
    docs_per_question: list[int] = []

    for qid, question in sorted(questions.items()):
        if str(question.get("question_type") or "") != args.question_type:
            continue
        docs = doc_ids(retrieval.get(qid), args.top_k)
        question_text = str(question.get("question", ""))
        answer_facts = [str(item) for item in question.get("answer_facts", []) if str(item)]
        packed_parts: list[str] = []
        for doc_id in docs:
            rel_path, title, content = cache.get(doc_id)
            snippet = packed_text(
                mode=args.mode,
                content=content,
                title=title,
                question=question_text,
                max_chars=args.max_chars_per_doc,
            )
            packed_parts.append(f"Title: {title}\nPath: {rel_path}\n{snippet}")
        coverage, full_coverage = fact_coverage(answer_facts, "\n\n".join(packed_parts))
        token_coverages.append(coverage)
        full_coverages.append(full_coverage)
        docs_per_question.append(len(docs))
        details.append(
            {
                "question_id": qid,
                "question": question_text,
                "retrieved_doc_count": len(docs),
                "fact_token_coverage_pct": coverage,
                "fact_full_coverage_pct": full_coverage,
                "document_ids": docs,
            }
        )

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.high_level_coverage.v1",
        "question_type": args.question_type,
        "questions": len(details),
        "retrieval_file": str(args.retrieval_file),
        "mode": args.mode,
        "top_k": args.top_k,
        "max_chars_per_doc": args.max_chars_per_doc,
        "average_retrieved_docs": mean([float(item) for item in docs_per_question]),
        "questions_with_docs": sum(1 for item in docs_per_question if item > 0),
        "average_fact_token_coverage_pct": mean(token_coverages),
        "average_fact_full_coverage_pct": mean(full_coverages),
        "thresholds": {
            "min_fact_token_coverage_pct": args.min_fact_token_coverage_pct,
        },
        "passed": mean(token_coverages) >= args.min_fact_token_coverage_pct,
        "note": "High-level questions have no expected_doc_ids, so this report measures answer-fact coverage instead of document recall.",
    }
    write_jsonl(args.output_jsonl, details)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--question-type", default="high_level")
    parser.add_argument("--mode", default="leading")
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--max-chars-per-doc", type=int, default=5000)
    parser.add_argument("--min-fact-token-coverage-pct", type=float, default=60.0)
    args = parser.parse_args()
    if args.top_k <= 0:
        parser.error("--top-k must be positive")
    if args.max_chars_per_doc <= 0:
        parser.error("--max-chars-per-doc must be positive")
    return args


if __name__ == "__main__":
    run(parse_args())
