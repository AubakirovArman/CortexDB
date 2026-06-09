#!/usr/bin/env python3
"""Generate an EnterpriseRAG evidence-table report from retrieved docs."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from evidence_table_extractor import (
    SCHEMA_VERSION,
    extract_document_content,
    extract_evidence_table,
    read_json,
    read_jsonl,
)


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def by_question_id(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {str(row.get("question_id")): row for row in rows if row.get("question_id")}


def build_table_for_question(
    *,
    question: dict[str, Any],
    retrieval: dict[str, Any],
    uuid_index: dict[str, str],
    sources_dir: Path,
    top_docs: int,
    max_facts_per_doc: int,
) -> dict[str, Any]:
    facts: list[dict[str, Any]] = []
    question_text = str(question.get("question") or "")
    doc_ids = [str(doc_id) for doc_id in retrieval.get("document_ids", [])][:top_docs]
    for doc_id in doc_ids:
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        facts.extend(
            extract_evidence_table(
                doc_id=doc_id,
                title=title,
                content=content,
                question=question_text,
                max_facts=max_facts_per_doc,
            )
        )
    facts.sort(key=lambda item: (-float(item["score"]), str(item["doc_id"]), int(item["line"])))
    return {
        "schema_version": SCHEMA_VERSION,
        "question_id": question.get("question_id"),
        "question_type": question.get("question_type"),
        "question": question.get("question"),
        "document_ids": doc_ids,
        "facts": facts,
        "fact_count": len(facts),
    }


def build_report(tables: list[dict[str, Any]], output_jsonl: Path, report_path: Path) -> dict[str, Any]:
    by_type = Counter(str(table.get("question_type") or "unknown") for table in tables)
    by_fact_type: Counter[str] = Counter()
    fact_counts: list[int] = []
    for table in tables:
        facts = table.get("facts", [])
        fact_counts.append(len(facts))
        for fact in facts:
            for fact_type in fact.get("fact_types", []):
                by_fact_type[str(fact_type)] += 1
    total_facts = sum(fact_counts)
    return {
        "schema_version": "cortexdb.enterprise_rag_bench.evidence_table_report.v1",
        "output_jsonl": str(output_jsonl),
        "output_report": str(report_path),
        "questions": len(tables),
        "total_facts": total_facts,
        "average_facts_per_question": round(total_facts / len(tables), 2) if tables else 0.0,
        "questions_without_facts": sum(1 for count in fact_counts if count == 0),
        "by_question_type": dict(sorted(by_type.items())),
        "by_fact_type": dict(sorted(by_fact_type.items())),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-docs", type=int, default=10)
    parser.add_argument("--max-facts-per-doc", type=int, default=6)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.top_docs <= 0:
        parser.error("--top-docs must be positive")
    if args.max_facts_per_doc <= 0:
        parser.error("--max-facts-per-doc must be positive")
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    args = parse_args()
    questions = read_jsonl(args.questions_file)
    if args.limit is not None:
        questions = questions[: args.limit]
    retrieval_by_id = by_question_id(read_jsonl(args.retrieval_file))
    uuid_index = read_json(args.uuid_index)
    tables = [
        build_table_for_question(
            question=question,
            retrieval=retrieval_by_id.get(str(question.get("question_id")), {}),
            uuid_index=uuid_index,
            sources_dir=args.sources_dir,
            top_docs=args.top_docs,
            max_facts_per_doc=args.max_facts_per_doc,
        )
        for question in questions
    ]
    report = build_report(tables, args.output_jsonl, args.report)
    write_jsonl(args.output_jsonl, tables)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
