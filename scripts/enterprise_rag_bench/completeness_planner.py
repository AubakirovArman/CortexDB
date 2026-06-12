#!/usr/bin/env python3
"""Build oracle-free completeness plans from clean retrieval artifacts.

The planner consumes only the user-visible question and retrieved document IDs.
It decomposes the question into answer sub-points, maps each sub-point to
materialized evidence spans, and emits an evidence-plan JSONL that can be fed
back into the official-clean answer runner.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from evidence_slot_planner import build_evidence_plan
from evidence_spans import select_evidence_spans
from question_decomposition import covered_unit_ids, evidence_units, normalize, tokens


SCHEMA_VERSION = "cortexdb.enterprise_rag_bench.completeness_plan.v1"


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


def clean_text(text: str, max_chars: int) -> str:
    cleaned = re.sub(r"\s+", " ", text).strip()
    if len(cleaned) <= max_chars:
        return cleaned
    return cleaned[: max(0, max_chars - 4)].rstrip() + " ..."


def unit_coverage(
    units: list[dict[str, Any]],
    span_text: str,
) -> list[str]:
    normalized = normalize(span_text)
    return covered_unit_ids(units, normalized, set(tokens(span_text)))


def build_completeness_plan(
    row: dict[str, Any],
    uuid_index: dict[str, str],
    sources_dir: Path,
    *,
    top_docs: int,
    max_spans_per_doc: int,
    max_chars_per_span: int,
) -> dict[str, Any]:
    question = str(row.get("question") or "")
    qid = str(row.get("question_id") or "")
    doc_ids = [str(item) for item in row.get("document_ids", []) or []]
    base_plan = build_evidence_plan({"question_id": qid, "question": question})
    units = evidence_units(question)
    coverage: dict[str, list[dict[str, Any]]] = {str(unit["id"]): [] for unit in units}
    mapped_spans: list[dict[str, Any]] = []

    for rank, doc_id in enumerate(doc_ids[:top_docs], 1):
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        spans = select_evidence_spans(
            content,
            question,
            max_spans=max_spans_per_doc,
            max_chars_per_span=max_chars_per_span,
        )
        for span_index, span in enumerate(spans, 1):
            covered_ids = unit_coverage(units, span.text)
            if not covered_ids:
                continue
            span_row = {
                "doc_id": doc_id,
                "doc_rank": rank,
                "span_index": span_index,
                "title": title,
                "score": round(span.score, 2),
                "signals": list(span.signals),
                "covered_unit_ids": covered_ids,
                "text": clean_text(span.text, max_chars_per_span),
            }
            mapped_spans.append(span_row)
            for unit_id in covered_ids:
                coverage.setdefault(unit_id, []).append(span_row)

    checklist: list[dict[str, Any]] = []
    covered_count = 0
    for unit in units:
        unit_id = str(unit["id"])
        evidence = sorted(
            coverage.get(unit_id, []),
            key=lambda item: (-float(item["score"]), int(item["doc_rank"]), int(item["span_index"])),
        )
        covered = bool(evidence)
        if covered:
            covered_count += 1
        checklist.append(
            {
                "id": unit_id,
                "kind": unit.get("kind"),
                "text": unit.get("text"),
                "covered": covered,
                "evidence_count": len(evidence),
                "evidence": evidence[:3],
            }
        )

    coverage_pct = round((covered_count * 100.0 / len(units)) if units else 0.0, 2)
    return {
        **base_plan,
        "schema_version": SCHEMA_VERSION,
        "planner": "completeness",
        "retrieved_doc_count": len(doc_ids),
        "planned_doc_count": min(len(doc_ids), top_docs),
        "checklist": checklist,
        "mapped_evidence_spans": mapped_spans[: top_docs * max_spans_per_doc],
        "covered_unit_count": covered_count,
        "total_unit_count": len(units),
        "coverage_pct": coverage_pct,
        "uncovered_unit_ids": [str(unit["id"]) for unit in checklist if not unit["covered"]],
        "repair_policy": (
            "Before final answer, verify every covered checklist item is answered. "
            "For uncovered requested items, state that the retrieved evidence does not show that part."
        ),
    }


def build_report(rows: list[dict[str, Any]], output_jsonl: Path, report_path: Path) -> dict[str, Any]:
    total_units = sum(int(row.get("total_unit_count", 0) or 0) for row in rows)
    covered_units = sum(int(row.get("covered_unit_count", 0) or 0) for row in rows)
    fully_covered = sum(1 for row in rows if row.get("total_unit_count") == row.get("covered_unit_count"))
    empty_mapping = sum(1 for row in rows if not row.get("mapped_evidence_spans"))
    return {
        "schema_version": "cortexdb.enterprise_rag_bench.completeness_plan_report.v1",
        "questions": len(rows),
        "output_jsonl": str(output_jsonl),
        "report": str(report_path),
        "total_units": total_units,
        "covered_units": covered_units,
        "average_coverage_pct": round((covered_units * 100.0 / total_units) if total_units else 0.0, 2),
        "fully_covered_questions": fully_covered,
        "empty_mapping_questions": empty_mapping,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-docs", type=int, default=10)
    parser.add_argument("--max-spans-per-doc", type=int, default=3)
    parser.add_argument("--max-chars-per-span", type=int, default=700)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.top_docs <= 0:
        parser.error("--top-docs must be positive")
    if args.max_spans_per_doc <= 0:
        parser.error("--max-spans-per-doc must be positive")
    if args.max_chars_per_span <= 0:
        parser.error("--max-chars-per-span must be positive")
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    args = parse_args()
    retrieval_rows = read_jsonl(args.retrieval_file)
    if args.limit is not None:
        retrieval_rows = retrieval_rows[: args.limit]
    uuid_index = read_json(args.uuid_index)
    rows = [
        build_completeness_plan(
            row,
            uuid_index,
            args.sources_dir,
            top_docs=args.top_docs,
            max_spans_per_doc=args.max_spans_per_doc,
            max_chars_per_span=args.max_chars_per_span,
        )
        for row in retrieval_rows
    ]
    report = build_report(rows, args.output_jsonl, args.report)
    write_jsonl(args.output_jsonl, rows)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
