#!/usr/bin/env python3
"""Evaluate local evidence packing for EnterpriseRAG-Bench without an LLM.

The script measures whether selected documents and local snippets/digests cover
gold answer facts. It does not call model APIs and does not generate answers.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from answer_context import brain_digest_context
from context_windows import query_tokens, question_aware_snippet
from evidence_digest import evidence_digest


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


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
    indexed: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in indexed:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        indexed[qid] = row
    return indexed


def extract_document_content(doc: dict[str, Any]) -> tuple[str, str]:
    title_field = doc.get("title_field_name")
    content_fields = doc.get("content_field_names")
    if not isinstance(title_field, str) or title_field not in doc:
        return ("", json.dumps(doc, ensure_ascii=False))
    title = str(doc.get(title_field, ""))
    if not isinstance(content_fields, list) or not content_fields:
        return (title, json.dumps(doc, ensure_ascii=False))
    parts: list[str] = []
    for field in content_fields:
        if not isinstance(field, str) or field not in doc:
            continue
        value = doc[field]
        if isinstance(value, list):
            value = "\n".join(str(item) for item in value)
        elif isinstance(value, dict):
            value = json.dumps(value, ensure_ascii=False)
        parts.append(f"{field}:\n{value}" if len(content_fields) > 1 else str(value))
    return (title, "\n\n".join(parts))


class DocumentCache:
    def __init__(self, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.values: dict[str, tuple[str, str, str]] = {}

    def get(self, doc_id: str) -> tuple[str, str, str]:
        cached = self.values.get(doc_id)
        if cached is not None:
            return cached
        rel_path = self.uuid_index.get(doc_id, "")
        if not rel_path:
            value = ("", "", "")
            self.values[doc_id] = value
            return value
        title, content = extract_document_content(read_json(self.sources_dir / rel_path))
        value = (rel_path, title, content)
        self.values[doc_id] = value
        return value


def packed_text(
    *,
    mode: str,
    content: str,
    title: str,
    question: str,
    max_chars: int,
) -> str:
    if mode == "leading":
        return content[:max_chars]
    if mode == "question-window":
        return question_aware_snippet(content, question, max_chars)
    if mode == "evidence-digest":
        return evidence_digest(content, title, question, max_chars)
    if mode == "brain-digest":
        return brain_digest_context(content, title, question, max_chars)
    if mode == "digest-window":
        digest = evidence_digest(content, title, question, max_chars=max(400, max_chars // 3))
        remaining = max(0, max_chars - len(digest) - 2)
        window = question_aware_snippet(content, question, remaining)
        return f"{digest}\n\n{window}".strip()[:max_chars]
    raise ValueError(f"unknown mode: {mode}")


def fact_coverage(answer_facts: list[str], text: str) -> tuple[float, float]:
    if not answer_facts:
        return (0.0, 0.0)
    text_tokens = query_tokens(text)
    coverage_values: list[float] = []
    full_count = 0
    for fact in answer_facts:
        fact_tokens = query_tokens(fact)
        if not fact_tokens:
            continue
        overlap = len(fact_tokens & text_tokens)
        coverage = overlap / len(fact_tokens)
        coverage_values.append(coverage)
        if coverage >= 0.6:
            full_count += 1
    if not coverage_values:
        return (0.0, 0.0)
    return (
        round(sum(coverage_values) / len(coverage_values) * 100.0, 2),
        round(full_count / len(coverage_values) * 100.0, 2),
    )


def recall_pct(expected: set[str], docs: list[str]) -> float | None:
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval = rows_by_id(read_jsonl(args.retrieval_file), "retrieval")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    cache = DocumentCache(uuid_index, args.sources_dir)

    detail_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    fact_coverages: list[float] = []
    full_fact_coverages: list[float] = []
    invalid_extra_counts: list[int] = []
    type_stats: dict[str, dict[str, float]] = defaultdict(lambda: defaultdict(float))
    type_counts: Counter[str] = Counter()

    for qid, question in questions.items():
        row = retrieval.get(qid, {"document_ids": []})
        docs = [str(item) for item in row.get("document_ids", []) if str(item)][: args.top_k]
        expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
        question_text = str(question.get("question", ""))
        answer_facts = [str(item) for item in question.get("answer_facts", []) if str(item)]
        qtype = str(question.get("question_type") or "unknown")
        recall = recall_pct(expected, docs)
        if recall is not None:
            recall_values.append(recall)
        invalid_extra = len([doc_id for doc_id in docs if doc_id not in expected])
        invalid_extra_counts.append(invalid_extra)

        packed_parts: list[str] = []
        gold_docs_in_pack = 0
        for doc_id in docs:
            rel_path, title, content = cache.get(doc_id)
            snippet = packed_text(
                mode=args.mode,
                content=content,
                title=title,
                question=question_text,
                max_chars=args.max_chars_per_doc,
            )
            if doc_id in expected:
                gold_docs_in_pack += 1
            packed_parts.append(f"Title: {title}\nPath: {rel_path}\n{snippet}")
        packed = "\n\n".join(packed_parts)
        coverage, full_coverage = fact_coverage(answer_facts, packed)
        if answer_facts:
            fact_coverages.append(coverage)
            full_fact_coverages.append(full_coverage)

        type_counts[qtype] += 1
        type_stats[qtype]["recall_sum"] += recall or 0.0
        type_stats[qtype]["fact_coverage_sum"] += coverage
        type_stats[qtype]["full_fact_coverage_sum"] += full_coverage
        type_stats[qtype]["invalid_extra_sum"] += invalid_extra

        detail_rows.append(
            {
                "question_id": qid,
                "question_type": qtype,
                "document_recall_pct": recall,
                "expected_doc_count": len(expected),
                "retrieved_doc_count": len(docs),
                "gold_docs_in_pack": gold_docs_in_pack,
                "invalid_extra_docs": invalid_extra,
                "fact_token_coverage_pct": coverage,
                "fact_full_coverage_pct": full_coverage,
            }
        )

    per_type: dict[str, Any] = {}
    for qtype, count in sorted(type_counts.items()):
        denominator = max(count, 1)
        per_type[qtype] = {
            "questions": count,
            "average_recall_pct": round(type_stats[qtype]["recall_sum"] / denominator, 2),
            "average_fact_token_coverage_pct": round(
                type_stats[qtype]["fact_coverage_sum"] / denominator,
                2,
            ),
            "average_fact_full_coverage_pct": round(
                type_stats[qtype]["full_fact_coverage_sum"] / denominator,
                2,
            ),
            "average_invalid_extra_docs": round(
                type_stats[qtype]["invalid_extra_sum"] / denominator,
                2,
            ),
        }

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.evidence_pack_eval.v1",
        "mode": args.mode,
        "retrieval_file": str(args.retrieval_file),
        "questions": len(detail_rows),
        "top_k": args.top_k,
        "max_chars_per_doc": args.max_chars_per_doc,
        "average_document_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "average_fact_token_coverage_pct": round(sum(fact_coverages) / len(fact_coverages), 2)
        if fact_coverages
        else 0.0,
        "average_fact_full_coverage_pct": round(
            sum(full_fact_coverages) / len(full_fact_coverages),
            2,
        )
        if full_fact_coverages
        else 0.0,
        "average_invalid_extra_docs": round(
            sum(invalid_extra_counts) / len(invalid_extra_counts),
            2,
        )
        if invalid_extra_counts
        else 0.0,
        "per_type": per_type,
    }
    write_jsonl(args.output_jsonl, detail_rows)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--mode", choices=["leading", "question-window", "evidence-digest", "brain-digest", "digest-window"], required=True)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--max-chars-per-doc", type=int, default=5000)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if args.top_k <= 0:
        parser.error("--top-k must be positive")
    if args.max_chars_per_doc <= 0:
        parser.error("--max-chars-per-doc must be positive")
    return args


if __name__ == "__main__":
    run(parse_args())
