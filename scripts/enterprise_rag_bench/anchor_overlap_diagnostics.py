#!/usr/bin/env python3
"""Offline anchor/evidence-overlap diagnostics for EnterpriseRAG retrieval."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from evidence_table_extractor import extract_document_content, read_json
from official_clean import read_jsonl, write_json

STOPWORDS = {
    "about",
    "after",
    "all",
    "and",
    "are",
    "default",
    "does",
    "for",
    "from",
    "have",
    "how",
    "into",
    "new",
    "not",
    "the",
    "their",
    "this",
    "total",
    "what",
    "when",
    "where",
    "which",
    "with",
}

TOKEN_RE = re.compile(r"[a-z0-9][a-z0-9_.:/-]{2,}", re.IGNORECASE)
EXACT_ANCHOR_RE = re.compile(
    r"\b[A-Z][A-Z0-9]{1,12}-\d+\b|`([^`]{2,120})`|/[A-Za-z0-9._~+\-/%]+|"
    r"\b[A-Za-z0-9_.-]+/[A-Za-z0-9._~+\-/%]+\b"
)


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = str(row.get("question_id") or "")
        if not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id {qid}")
        values[qid] = row
    return values


def expected_doc_ids(question: dict[str, Any]) -> list[str]:
    seen: set[str] = set()
    values: list[str] = []
    for item in question.get("expected_doc_ids", []) or []:
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            seen.add(doc_id)
            values.append(doc_id)
    return values


def retrieved_doc_ids(row: dict[str, Any], top_k: int) -> list[str]:
    seen: set[str] = set()
    values: list[str] = []
    for item in row.get("document_ids", []) or []:
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            seen.add(doc_id)
            values.append(doc_id)
        if len(values) >= top_k:
            break
    return values


def query_anchors(question: str) -> list[str]:
    anchors: list[str] = []
    for match in EXACT_ANCHOR_RE.finditer(question):
        value = match.group(1) or match.group(0)
        value = value.strip(" `,.;:()[]{}")
        if value and value not in anchors:
            anchors.append(value)
    for token in TOKEN_RE.findall(question.casefold()):
        token = token.strip("._:/-")
        if len(token) < 4 or token in STOPWORDS:
            continue
        if token.isdigit():
            continue
        if token not in anchors:
            anchors.append(token)
    return anchors[:40]


def load_doc_text(doc_id: str, uuid_index: dict[str, str], sources_dir: Path) -> str:
    rel_path = uuid_index.get(doc_id)
    if not rel_path:
        return ""
    try:
        title, content = extract_document_content(read_json(sources_dir / rel_path))
    except Exception:
        return ""
    return f"{title}\n{content}".casefold()


def overlap_count(anchors: list[str], text: str) -> int:
    haystack = text.casefold()
    return sum(1 for anchor in anchors if anchor.casefold() in haystack)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval = rows_by_id(read_jsonl(args.retrieval_file), "retrieval")
    uuid_index = read_json(args.uuid_index)

    details: list[dict[str, Any]] = []
    recall_values: list[float] = []
    invalid_values: list[int] = []
    overlap_ratios: list[float] = []
    strong_overlap_ratios: list[float] = []
    invalid_without_overlap = 0
    invalid_with_overlap = 0
    invalid_without_strong_overlap = 0
    invalid_with_strong_overlap = 0
    gold_without_overlap = 0
    gold_without_strong_overlap = 0
    total_gold_hits = 0

    for qid, question in sorted(questions.items()):
        row = retrieval.get(qid, {})
        expected = set(expected_doc_ids(question))
        retrieved = retrieved_doc_ids(row, args.top_k)
        anchors = query_anchors(str(question.get("question") or row.get("question") or ""))
        hits = len(expected & set(retrieved))
        invalid = [doc_id for doc_id in retrieved if doc_id not in expected]
        if expected:
            recall_values.append(hits / len(expected) * 100.0)
        invalid_values.append(len(invalid))

        doc_details: list[dict[str, Any]] = []
        overlapped_docs = 0
        strong_overlapped_docs = 0
        for rank, doc_id in enumerate(retrieved, 1):
            count = overlap_count(anchors, load_doc_text(doc_id, uuid_index, args.sources_dir))
            has_overlap = count > 0
            has_strong_overlap = count >= args.strong_overlap_min_anchors
            if has_overlap:
                overlapped_docs += 1
            if has_strong_overlap:
                strong_overlapped_docs += 1
            is_expected = doc_id in expected
            if is_expected:
                total_gold_hits += 1
                if not has_overlap:
                    gold_without_overlap += 1
                if not has_strong_overlap:
                    gold_without_strong_overlap += 1
            elif has_overlap:
                invalid_with_overlap += 1
                if has_strong_overlap:
                    invalid_with_strong_overlap += 1
                else:
                    invalid_without_strong_overlap += 1
            else:
                invalid_without_overlap += 1
                invalid_without_strong_overlap += 1
            doc_details.append(
                {
                    "rank": rank,
                    "doc_id": doc_id,
                    "expected": is_expected,
                    "overlap_anchor_count": count,
                    "strong_overlap": has_strong_overlap,
                }
            )
        overlap_ratio = overlapped_docs / len(retrieved) * 100.0 if retrieved else 0.0
        strong_overlap_ratio = (
            strong_overlapped_docs / len(retrieved) * 100.0 if retrieved else 0.0
        )
        overlap_ratios.append(overlap_ratio)
        strong_overlap_ratios.append(strong_overlap_ratio)
        details.append(
            {
                "question_id": qid,
                "question_type": question.get("question_type"),
                "anchor_count": len(anchors),
                "anchors": anchors[: args.max_detail_anchors],
                "expected_doc_count": len(expected),
                "retrieved_doc_count": len(retrieved),
                "hit_doc_count": hits,
                "document_recall_pct": round(hits / len(expected) * 100.0, 2)
                if expected
                else None,
                "invalid_extra_docs": len(invalid),
                "overlap_doc_pct": round(overlap_ratio, 2),
                "strong_overlap_doc_pct": round(strong_overlap_ratio, 2),
                "docs": doc_details if args.include_details else [],
            }
        )

    avg_recall = sum(recall_values) / len(recall_values) if recall_values else 0.0
    avg_invalid = sum(invalid_values) / len(invalid_values) if invalid_values else 0.0
    avg_overlap = sum(overlap_ratios) / len(overlap_ratios) if overlap_ratios else 0.0
    avg_strong_overlap = (
        sum(strong_overlap_ratios) / len(strong_overlap_ratios)
        if strong_overlap_ratios
        else 0.0
    )
    failures: list[str] = []
    if avg_recall < args.min_recall_pct:
        failures.append(f"average_recall_pct {avg_recall:.2f} < {args.min_recall_pct:.2f}")
    if avg_invalid > args.max_average_invalid_extra_docs:
        failures.append(
            "average_invalid_extra_docs "
            f"{avg_invalid:.2f} > {args.max_average_invalid_extra_docs:.2f}"
        )
    if avg_overlap < args.min_overlap_doc_pct:
        failures.append(f"average_overlap_doc_pct {avg_overlap:.2f} < {args.min_overlap_doc_pct:.2f}")
    if avg_strong_overlap < args.min_strong_overlap_doc_pct:
        failures.append(
            "average_strong_overlap_doc_pct "
            f"{avg_strong_overlap:.2f} < {args.min_strong_overlap_doc_pct:.2f}"
        )

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.anchor_overlap_diagnostics.v1",
        "status": "failed" if failures else "passed",
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "top_k": args.top_k,
        "questions": len(questions),
        "average_recall_pct": round(avg_recall, 2),
        "average_invalid_extra_docs": round(avg_invalid, 2),
        "average_overlap_doc_pct": round(avg_overlap, 2),
        "average_strong_overlap_doc_pct": round(avg_strong_overlap, 2),
        "strong_overlap_min_anchors": args.strong_overlap_min_anchors,
        "invalid_with_overlap": invalid_with_overlap,
        "invalid_without_overlap": invalid_without_overlap,
        "invalid_with_strong_overlap": invalid_with_strong_overlap,
        "invalid_without_strong_overlap": invalid_without_strong_overlap,
        "gold_hits_without_overlap": gold_without_overlap,
        "gold_hits_without_strong_overlap": gold_without_strong_overlap,
        "total_gold_hits": total_gold_hits,
        "thresholds": {
            "min_recall_pct": args.min_recall_pct,
            "max_average_invalid_extra_docs": args.max_average_invalid_extra_docs,
            "min_overlap_doc_pct": args.min_overlap_doc_pct,
            "min_strong_overlap_doc_pct": args.min_strong_overlap_doc_pct,
        },
        "failures": failures,
        "details": details if args.include_details else [],
    }
    write_json(args.report, report)
    if args.details_jsonl:
        args.details_jsonl.parent.mkdir(parents=True, exist_ok=True)
        with args.details_jsonl.open("w", encoding="utf-8") as handle:
            for row in details:
                handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--details-jsonl", type=Path)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--min-recall-pct", type=float, default=60.0)
    parser.add_argument("--max-average-invalid-extra-docs", type=float, default=10.0)
    parser.add_argument("--min-overlap-doc-pct", type=float, default=35.0)
    parser.add_argument("--strong-overlap-min-anchors", type=int, default=2)
    parser.add_argument("--min-strong-overlap-doc-pct", type=float, default=35.0)
    parser.add_argument("--max-detail-anchors", type=int, default=12)
    parser.add_argument("--include-details", action="store_true")
    args = parser.parse_args()
    report = build_report(args)
    print(json.dumps(report, sort_keys=True))
    return 1 if report["status"] != "passed" else 0


if __name__ == "__main__":
    raise SystemExit(main())
