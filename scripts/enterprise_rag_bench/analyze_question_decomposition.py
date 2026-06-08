#!/usr/bin/env python3
"""Explain EnterpriseRAG questions as retrieval evidence units."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from hybrid_rerank_features import DocumentCache, read_json, read_jsonl
from question_decomposition import covered_unit_ids, evidence_units


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_md(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# EnterpriseRAG Question Decomposition Report",
        "",
        f"- questions: {report['questions']}",
        f"- retrieval_file: {report.get('retrieval_file') or 'none'}",
        f"- average_unit_coverage_pct: {report['average_unit_coverage_pct']}",
        "",
        "| question_id | type | units | covered | missing | expected_docs |",
        "| --- | --- | ---: | ---: | ---: | ---: |",
    ]
    for row in report["rows"]:
        lines.append(
            "| {question_id} | {question_type} | {unit_count} | {covered_unit_count} | "
            "{missing_unit_count} | {expected_doc_count} |".format(**row)
        )
    lines.append("")
    lines.append("## Top Missing Units")
    for row in sorted(report["rows"], key=lambda item: (-item["missing_unit_count"], item["question_id"]))[:20]:
        if not row["missing_units"]:
            continue
        lines.append("")
        lines.append(f"### {row['question_id']} ({row['question_type']})")
        lines.append(row["question"])
        for unit in row["missing_units"][:8]:
            lines.append(f"- `{unit['id']}` {unit['kind']}: {unit['text']}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def rows_by_id(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {
        str(row["question_id"]): row
        for row in rows
        if isinstance(row.get("question_id"), str)
    }


def analyze(args: argparse.Namespace) -> dict[str, Any]:
    question_rows = read_jsonl(args.questions_file)
    if args.limit is not None:
        question_rows = question_rows[: args.limit]

    retrieval_rows: dict[str, dict[str, Any]] = {}
    doc_cache: DocumentCache | None = None
    if args.retrieval_file:
        retrieval_rows = rows_by_id(read_jsonl(args.retrieval_file))
        if not args.uuid_index or not args.sources_dir:
            raise ValueError("--uuid-index and --sources-dir are required with --retrieval-file")
        doc_cache = DocumentCache(read_json(args.uuid_index), args.sources_dir)

    rows: list[dict[str, Any]] = []
    coverage_values: list[float] = []
    for question in question_rows:
        qid = str(question.get("question_id"))
        units = evidence_units(str(question.get("question", "")))
        unit_by_id = {str(unit["id"]): unit for unit in units}
        covered: set[str] = set()
        doc_coverages: list[dict[str, Any]] = []

        retrieval = retrieval_rows.get(qid, {})
        doc_ids = [str(item) for item in retrieval.get("document_ids", [])][: args.top_docs]
        if doc_cache is not None:
            for doc_id in doc_ids:
                doc = doc_cache.get(doc_id)
                doc_units = covered_unit_ids(units, doc["normalized"], doc["token_set"])
                covered.update(doc_units)
                doc_coverages.append(
                    {
                        "doc_id": doc_id,
                        "covered_unit_ids": doc_units,
                        "covered_unit_count": len(doc_units),
                    }
                )

        coverage_pct = len(covered) / len(units) * 100.0 if units else 100.0
        coverage_values.append(coverage_pct)
        missing = [unit for unit in units if str(unit["id"]) not in covered]
        rows.append(
            {
                "question_id": qid,
                "question_type": question.get("question_type"),
                "question": question.get("question"),
                "expected_doc_count": len(question.get("expected_doc_ids", []) or []),
                "answer_fact_count": len(question.get("answer_facts", []) or []),
                "unit_count": len(units),
                "covered_unit_count": len(covered),
                "missing_unit_count": len(missing),
                "unit_coverage_pct": round(coverage_pct, 2),
                "units": units,
                "covered_unit_ids": sorted(covered),
                "missing_units": missing,
                "doc_coverages": doc_coverages,
                "covered_unit_texts": [unit_by_id[unit_id]["text"] for unit_id in sorted(covered) if unit_id in unit_by_id],
            }
        )

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.question_decomposition_report.v1",
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file) if args.retrieval_file else None,
        "output_json": str(args.output_json),
        "output_md": str(args.output_md) if args.output_md else None,
        "questions": len(rows),
        "top_docs": args.top_docs,
        "average_unit_coverage_pct": round(sum(coverage_values) / len(coverage_values), 2) if coverage_values else 0.0,
        "rows": rows,
    }
    write_json(args.output_json, report)
    if args.output_md:
        write_md(args.output_md, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path)
    parser.add_argument("--uuid-index", type=Path)
    parser.add_argument("--sources-dir", type=Path)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-md", type=Path)
    parser.add_argument("--top-docs", type=int, default=12)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.top_docs <= 0:
        parser.error("--top-docs must be positive")
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    report = analyze(parse_args())
    print(
        json.dumps(
            {
                "questions": report["questions"],
                "average_unit_coverage_pct": report["average_unit_coverage_pct"],
                "output_json": report["output_json"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
