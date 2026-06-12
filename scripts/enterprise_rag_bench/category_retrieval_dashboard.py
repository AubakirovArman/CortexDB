#!/usr/bin/env python3
"""Build a per-category EnterpriseRAG retrieval dashboard with commit history."""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import time
from collections import defaultdict
from pathlib import Path
from typing import Any

from official_clean import assert_clean_retrieval, read_jsonl, write_json

QUESTION_TYPES = [
    "basic",
    "semantic",
    "intra_document_reasoning",
    "project_related",
    "constrained",
    "conflicting_info",
    "completeness",
    "high_level",
    "info_not_found",
    "miscellaneous",
]


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id {qid}")
        values[qid] = row
    return values


def doc_ids(row: dict[str, Any] | None, limit: int) -> list[str]:
    if not row:
        return []
    values: list[str] = []
    seen: set[str] = set()
    for item in row.get("document_ids", []):
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            values.append(doc_id)
            seen.add(doc_id)
        if len(values) >= limit:
            break
    return values


def expected_doc_ids(question: dict[str, Any]) -> list[str]:
    values: list[str] = []
    seen: set[str] = set()
    for item in question.get("expected_doc_ids", []):
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            values.append(doc_id)
            seen.add(doc_id)
    return values


def reciprocal_rank(expected: set[str], retrieved: list[str]) -> float:
    for index, doc_id in enumerate(retrieved, 1):
        if doc_id in expected:
            return 1.0 / index
    return 0.0


def dcg(expected: set[str], retrieved: list[str]) -> float:
    score = 0.0
    for index, doc_id in enumerate(retrieved, 1):
        if doc_id in expected:
            score += 1.0 / math.log2(index + 1)
    return score


def ndcg(expected: set[str], retrieved: list[str]) -> float:
    if not expected:
        return 0.0
    ideal_len = min(len(expected), len(retrieved))
    if ideal_len == 0:
        return 0.0
    ideal = sum(1.0 / math.log2(index + 1) for index in range(1, ideal_len + 1))
    return dcg(expected, retrieved) / ideal if ideal else 0.0


def mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def round2(value: float) -> float:
    return round(value, 2)


class CategoryStats:
    def __init__(self) -> None:
        self.questions = 0
        self.answerable_questions = 0
        self.recall: list[float] = []
        self.precision: list[float] = []
        self.mrr: list[float] = []
        self.ndcg: list[float] = []
        self.invalid: list[float] = []
        self.hit_questions = 0
        self.full_recall_questions = 0

    def record(self, expected: list[str], retrieved: list[str]) -> None:
        self.questions += 1
        expected_set = set(expected)
        retrieved_set = set(retrieved)
        hits = len(expected_set & retrieved_set)
        invalid = len([doc_id for doc_id in retrieved if doc_id not in expected_set])
        self.invalid.append(float(invalid))
        if not expected:
            return
        self.answerable_questions += 1
        recall = hits / len(expected_set) * 100.0
        precision = hits / len(retrieved) * 100.0 if retrieved else 0.0
        self.recall.append(recall)
        self.precision.append(precision)
        self.mrr.append(reciprocal_rank(expected_set, retrieved))
        self.ndcg.append(ndcg(expected_set, retrieved))
        if hits > 0:
            self.hit_questions += 1
        if hits == len(expected_set):
            self.full_recall_questions += 1

    def as_json(self) -> dict[str, Any]:
        return {
            "questions": self.questions,
            "answerable_questions": self.answerable_questions,
            "average_recall_pct": round2(mean(self.recall)),
            "average_precision_pct": round2(mean(self.precision)),
            "hit_questions": self.hit_questions,
            "full_recall_questions": self.full_recall_questions,
            "average_invalid_extra_docs": round2(mean(self.invalid)),
            "mrr": round2(mean(self.mrr)),
            "ndcg": round2(mean(self.ndcg)),
        }


def current_git_commit() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:
        return "unknown"


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval_rows = read_jsonl(args.retrieval_file)
    assert_clean_retrieval(retrieval_rows)
    retrieval = rows_by_id(retrieval_rows, "retrieval")
    commit = args.commit or current_git_commit()
    run_id = args.run_id or f"{commit}-{int(time.time())}"

    by_type = {qtype: CategoryStats() for qtype in QUESTION_TYPES}
    by_type["unknown"] = CategoryStats()
    details: list[dict[str, Any]] = []
    missing_rows: list[str] = []

    for qid, question in sorted(questions.items()):
        qtype = str(question.get("question_type") or "unknown")
        stats = by_type.setdefault(qtype, CategoryStats())
        expected = expected_doc_ids(question)
        row = retrieval.get(qid)
        if row is None:
            missing_rows.append(qid)
        retrieved = doc_ids(row, args.top_k)
        stats.record(expected, retrieved)
        expected_set = set(expected)
        hits = len(expected_set & set(retrieved))
        details.append(
            {
                "question_id": qid,
                "question_type": qtype,
                "expected_doc_count": len(expected),
                "retrieved_doc_count": len(retrieved),
                "hit_doc_count": hits,
                "invalid_extra_docs": len(
                    [doc_id for doc_id in retrieved if doc_id not in expected_set]
                ),
            }
        )

    categories = {
        qtype: stats.as_json()
        for qtype, stats in sorted(by_type.items())
        if stats.questions > 0 or qtype in QUESTION_TYPES
    }
    totals = CategoryStats()
    for detail in details:
        qid = str(detail["question_id"])
        question = questions[qid]
        totals.record(expected_doc_ids(question), doc_ids(retrieval.get(qid), args.top_k))

    history_rows = read_history(args.history)
    trend = trend_summary(history_rows, categories)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.category_retrieval_dashboard.v1",
        "status": "passed" if not missing_rows else "failed",
        "run_id": run_id,
        "commit": commit,
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "top_k": args.top_k,
        "categories_expected": QUESTION_TYPES,
        "category_count": len(QUESTION_TYPES),
        "metrics": totals.as_json(),
        "per_category": categories,
        "trend_vs_previous": trend,
        "missing_retrieval_question_ids": missing_rows,
        "details": details if args.include_details else [],
    }
    append_history(args.history, history_row(report))
    return report


def read_history(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            rows.append(json.loads(line))
    return rows


def append_history(path: Path | None, row: dict[str, Any]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def history_row(report: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "cortexdb.enterprise_rag_bench.category_retrieval_history.v1",
        "run_id": report["run_id"],
        "commit": report["commit"],
        "top_k": report["top_k"],
        "metrics": report["metrics"],
        "per_category": report["per_category"],
    }


def trend_summary(history_rows: list[dict[str, Any]], categories: dict[str, Any]) -> dict[str, Any]:
    if not history_rows:
        return {"baseline": "none", "per_category": {}}
    previous = history_rows[-1]
    previous_categories = previous.get("per_category", {})
    trends: dict[str, Any] = {}
    for qtype, current in categories.items():
        previous_row = previous_categories.get(qtype, {})
        trends[qtype] = {
            "average_recall_delta_pct": round2(
                float(current.get("average_recall_pct", 0.0))
                - float(previous_row.get("average_recall_pct", 0.0))
            ),
            "average_precision_delta_pct": round2(
                float(current.get("average_precision_pct", 0.0))
                - float(previous_row.get("average_precision_pct", 0.0))
            ),
            "invalid_extra_delta": round2(
                float(current.get("average_invalid_extra_docs", 0.0))
                - float(previous_row.get("average_invalid_extra_docs", 0.0))
            ),
            "mrr_delta": round2(float(current.get("mrr", 0.0)) - float(previous_row.get("mrr", 0.0))),
            "ndcg_delta": round2(
                float(current.get("ndcg", 0.0)) - float(previous_row.get("ndcg", 0.0))
            ),
        }
    return {
        "baseline": {
            "run_id": previous.get("run_id"),
            "commit": previous.get("commit"),
        },
        "per_category": trends,
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Category Retrieval Dashboard",
        "",
        f"- status: `{report['status']}`",
        f"- run_id: `{report['run_id']}`",
        f"- commit: `{report['commit']}`",
        f"- questions: `{report['questions_file']}`",
        f"- retrieval: `{report['retrieval_file']}`",
        f"- top_k: `{report['top_k']}`",
        "",
        "## Overall",
        "",
        "| Metric | Value |",
        "| --- | ---: |",
    ]
    for key, value in report["metrics"].items():
        lines.append(f"| `{key}` | {value} |")
    lines.extend(
        [
            "",
            "## Per Category",
            "",
            "| Category | Qs | Answerable | Recall | Precision | Hit | Full | Invalid | MRR | nDCG |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for qtype in QUESTION_TYPES:
        row = report["per_category"].get(qtype, {})
        lines.append(
            f"| `{qtype}` | {row.get('questions', 0)} | {row.get('answerable_questions', 0)} | "
            f"{row.get('average_recall_pct', 0.0)} | {row.get('average_precision_pct', 0.0)} | "
            f"{row.get('hit_questions', 0)} | {row.get('full_recall_questions', 0)} | "
            f"{row.get('average_invalid_extra_docs', 0.0)} | {row.get('mrr', 0.0)} | "
            f"{row.get('ndcg', 0.0)} |"
        )
    if report["missing_retrieval_question_ids"]:
        lines.extend(["", "## Missing Retrieval Rows", ""])
        lines.extend(f"- `{qid}`" for qid in report["missing_retrieval_question_ids"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--history", type=Path)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--run-id")
    parser.add_argument("--commit")
    parser.add_argument("--include-details", action="store_true")
    args = parser.parse_args()
    if args.top_k <= 0:
        parser.error("--top-k must be positive")
    return args


def main() -> int:
    args = parse_args()
    report = build_report(args)
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    print(
        json.dumps(
            {
                "status": report["status"],
                "category_count": report["category_count"],
                "answerable_questions": report["metrics"]["answerable_questions"],
                "average_recall_pct": report["metrics"]["average_recall_pct"],
                "average_precision_pct": report["metrics"]["average_precision_pct"],
                "average_invalid_extra_docs": report["metrics"]["average_invalid_extra_docs"],
                "output": str(args.report),
            },
            sort_keys=True,
        )
    )
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
