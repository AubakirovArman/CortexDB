#!/usr/bin/env python3
"""Analyze EnterpriseRAG-Bench answer failures from local artifacts."""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any


STOPWORDS = {
    "a",
    "an",
    "and",
    "are",
    "as",
    "be",
    "by",
    "for",
    "from",
    "if",
    "in",
    "is",
    "it",
    "must",
    "of",
    "on",
    "or",
    "should",
    "that",
    "the",
    "to",
    "was",
    "with",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def tokens(text: str) -> set[str]:
    return {
        token
        for token in re.findall(r"[a-zA-Z0-9_]+", text.lower())
        if len(token) > 1 and token not in STOPWORDS
    }


def fact_overlap(answer: str, facts: list[str]) -> dict[str, Any]:
    answer_tokens = tokens(answer)
    if not facts:
        return {"mean_pct": 100.0, "facts": []}
    rows = []
    for fact in facts:
        fact_tokens = tokens(fact)
        overlap = fact_tokens & answer_tokens
        pct = len(overlap) / len(fact_tokens) * 100 if fact_tokens else 100.0
        rows.append(
            {
                "fact": fact,
                "token_overlap_pct": round(pct, 2),
                "missing_terms": sorted(fact_tokens - answer_tokens)[:12],
            }
        )
    mean = sum(row["token_overlap_pct"] for row in rows) / len(rows)
    return {"mean_pct": round(mean, 2), "facts": rows}


def load_by_id(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {str(row.get("question_id")): row for row in rows if row.get("question_id")}


def classify(row: dict[str, Any]) -> str:
    if not row["answer_nonempty"]:
        return "empty_answer"
    if row["document_recall_pct"] in (None, 0):
        return "retrieval_miss"
    if row["answer_contains_insufficient_information"]:
        return "abstained_with_evidence"
    if row["fact_token_overlap_pct"] >= 75:
        return "likely_judge_or_format_issue"
    return "answer_missing_gold_facts"


def analyze(args: argparse.Namespace) -> dict[str, Any]:
    questions = load_by_id(read_jsonl(args.questions_file))
    answers = load_by_id(read_jsonl(args.answers_file))
    metrics = read_json(args.metrics_file)
    metric_rows = load_by_id(metrics.get("questions", []))

    rows: list[dict[str, Any]] = []
    buckets: dict[str, int] = {}
    for qid, question in sorted(questions.items()):
        answer = answers.get(qid, {})
        metric = metric_rows.get(qid, {})
        answer_text = str(answer.get("answer", ""))
        expected_docs = [str(doc_id) for doc_id in question.get("expected_doc_ids", [])]
        retrieved_docs = [str(doc_id) for doc_id in answer.get("document_ids", [])]
        overlap = fact_overlap(answer_text, [str(f) for f in question.get("answer_facts", [])])
        row = {
            "question_id": qid,
            "question_type": question.get("question_type"),
            "answer_nonempty": bool(answer_text.strip()),
            "answer_chars": len(answer_text),
            "answer_contains_insufficient_information": "insufficient information" in answer_text.lower(),
            "expected_doc_count": len(expected_docs),
            "retrieved_doc_count": len(retrieved_docs),
            "all_expected_docs_retrieved": set(expected_docs).issubset(set(retrieved_docs)),
            "document_recall_pct": metric.get("document_recall_pct"),
            "invalid_extra_docs": metric.get("invalid_extra_docs"),
            "answer_correct": bool(metric.get("answer_correct")),
            "completeness_pct": float(metric.get("completeness_pct") or 0.0),
            "correctness_reasoning_blank": not bool(str(metric.get("correctness_reasoning", "")).strip()),
            "fact_token_overlap_pct": overlap["mean_pct"],
            "fact_overlap": overlap["facts"],
            "gold_answer": question.get("gold_answer", ""),
            "candidate_answer_preview": answer_text[:500],
        }
        row["bucket"] = classify(row)
        buckets[row["bucket"]] = buckets.get(row["bucket"], 0) + 1
        rows.append(row)

    nonempty = sum(1 for row in rows if row["answer_nonempty"])
    doc_hit_answer_fail = sum(
        1
        for row in rows
        if (row["document_recall_pct"] or 0) > 0 and not row["answer_correct"]
    )
    blank_reasoning = sum(1 for row in rows if row["correctness_reasoning_blank"])
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.answer_error_analysis.v1",
        "questions": len(rows),
        "answers_file": str(args.answers_file),
        "metrics_file": str(args.metrics_file),
        "nonempty_answers": nonempty,
        "doc_hit_but_answer_correct_false": doc_hit_answer_fail,
        "blank_correctness_reasoning": blank_reasoning,
        "judge_env": {
            "LLM_PROVIDER": os.environ.get("LLM_PROVIDER", "openai"),
            "LLM_API_KEY_present": bool(os.environ.get("LLM_API_KEY")),
            "LLM_MODEL_NAME_present": bool(os.environ.get("LLM_MODEL_NAME")),
        },
        "buckets": buckets,
        "aggregate_stats": metrics.get("aggregate_stats", {}),
        "question_type_stats": metrics.get("question_type_stats", {}),
        "top_failures": sorted(
            rows,
            key=lambda row: (
                row["answer_correct"],
                -(row["document_recall_pct"] or 0),
                -row["fact_token_overlap_pct"],
                row["question_id"],
            ),
        )[: args.top_failures],
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--answers-file", type=Path, required=True)
    parser.add_argument("--metrics-file", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-failures", type=int, default=12)
    args = parser.parse_args()
    if args.top_failures <= 0:
        parser.error("--top-failures must be positive")
    return args


def main() -> int:
    print(json.dumps(analyze(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
