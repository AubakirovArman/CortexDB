#!/usr/bin/env python3
"""Fail promotion when EnterpriseRAG retrieval regresses by category."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from category_retrieval_dashboard import CategoryStats, QUESTION_TYPES, doc_ids, expected_doc_ids, rows_by_id
from official_clean import assert_clean_retrieval, read_jsonl, write_json


def build_metrics(
    questions: dict[str, dict[str, Any]],
    retrieval: dict[str, dict[str, Any]],
    top_k: int,
) -> tuple[dict[str, Any], dict[str, Any]]:
    per_category = {qtype: CategoryStats() for qtype in QUESTION_TYPES}
    per_category["unknown"] = CategoryStats()
    overall = CategoryStats()
    missing_rows: list[str] = []

    for qid, question in sorted(questions.items()):
        qtype = str(question.get("question_type") or "unknown")
        expected = expected_doc_ids(question)
        row = retrieval.get(qid)
        if row is None:
            missing_rows.append(qid)
        retrieved = doc_ids(row, top_k)
        per_category.setdefault(qtype, CategoryStats()).record(expected, retrieved)
        overall.record(expected, retrieved)

    return (
        overall.as_json(),
        {
            qtype: stats.as_json()
            for qtype, stats in sorted(per_category.items())
            if stats.questions > 0 or qtype in QUESTION_TYPES
        }
        | {"_missing_retrieval_question_ids": missing_rows},
    )


def delta(candidate: float | int, baseline: float | int) -> float:
    return round(float(candidate) - float(baseline), 2)


def check_metric(
    *,
    errors: list[str],
    category: str,
    metric: str,
    baseline: float | int,
    candidate: float | int,
    max_regression: float,
) -> float:
    value = delta(candidate, baseline)
    if value < -max_regression:
        errors.append(
            f"{category}.{metric} regressed {abs(value)} > {max_regression} "
            f"(baseline={baseline}, candidate={candidate})"
        )
    return value


def check_invalid_metric(
    *,
    errors: list[str],
    category: str,
    baseline: float | int,
    candidate: float | int,
    max_regression: float,
) -> float:
    value = delta(candidate, baseline)
    if value > max_regression:
        errors.append(
            f"{category}.average_invalid_extra_docs regressed {value} > {max_regression} "
            f"(baseline={baseline}, candidate={candidate})"
        )
    return value


def compare_categories(
    baseline: dict[str, Any],
    candidate: dict[str, Any],
    args: argparse.Namespace,
) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    comparison: dict[str, Any] = {}
    for category in QUESTION_TYPES:
        before = baseline.get(category, {})
        after = candidate.get(category, {})
        answerable = int(max(before.get("answerable_questions", 0), after.get("answerable_questions", 0)))
        row = {
            "baseline": before,
            "candidate": after,
            "delta": {
                "average_invalid_extra_docs": check_invalid_metric(
                    errors=errors,
                    category=category,
                    baseline=before.get("average_invalid_extra_docs", 0.0),
                    candidate=after.get("average_invalid_extra_docs", 0.0),
                    max_regression=args.max_category_invalid_extra_regression,
                )
            },
        }
        if answerable > 0:
            row["delta"].update(
                {
                    "average_recall_pct": check_metric(
                        errors=errors,
                        category=category,
                        metric="average_recall_pct",
                        baseline=before.get("average_recall_pct", 0.0),
                        candidate=after.get("average_recall_pct", 0.0),
                        max_regression=args.max_category_recall_regression_pct,
                    ),
                    "average_precision_pct": check_metric(
                        errors=errors,
                        category=category,
                        metric="average_precision_pct",
                        baseline=before.get("average_precision_pct", 0.0),
                        candidate=after.get("average_precision_pct", 0.0),
                        max_regression=args.max_category_precision_regression_pct,
                    ),
                    "mrr": check_metric(
                        errors=errors,
                        category=category,
                        metric="mrr",
                        baseline=before.get("mrr", 0.0),
                        candidate=after.get("mrr", 0.0),
                        max_regression=args.max_category_mrr_regression,
                    ),
                    "ndcg": check_metric(
                        errors=errors,
                        category=category,
                        metric="ndcg",
                        baseline=before.get("ndcg", 0.0),
                        candidate=after.get("ndcg", 0.0),
                        max_regression=args.max_category_ndcg_regression,
                    ),
                }
            )
        comparison[category] = row
    return comparison, errors


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Per-category Regression Gate",
        "",
        f"- status: `{report['status']}`",
        f"- questions: `{report['questions_file']}`",
        f"- baseline: `{report['baseline_retrieval_file']}`",
        f"- candidate: `{report['candidate_retrieval_file']}`",
        f"- top_k: `{report['top_k']}`",
        "",
        "## Per Category",
        "",
        "| Category | Recall Δ | Precision Δ | Invalid Δ | MRR Δ | nDCG Δ |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for category in QUESTION_TYPES:
        row = report["per_category"][category]["delta"]
        lines.append(
            f"| `{category}` | {row.get('average_recall_pct', 'n/a')} | "
            f"{row.get('average_precision_pct', 'n/a')} | "
            f"{row.get('average_invalid_extra_docs', 'n/a')} | "
            f"{row.get('mrr', 'n/a')} | {row.get('ndcg', 'n/a')} |"
        )
    if report["errors"]:
        lines.extend(["", "## Errors", ""])
        lines.extend(f"- {error}" for error in report["errors"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline_rows = read_jsonl(args.baseline_retrieval_file)
    candidate_rows = read_jsonl(args.candidate_retrieval_file)
    assert_clean_retrieval(baseline_rows)
    assert_clean_retrieval(candidate_rows)
    baseline = rows_by_id(baseline_rows, "baseline")
    candidate = rows_by_id(candidate_rows, "candidate")

    baseline_metrics, baseline_categories = build_metrics(questions, baseline, args.top_k)
    candidate_metrics, candidate_categories = build_metrics(questions, candidate, args.top_k)
    missing = (
        baseline_categories.pop("_missing_retrieval_question_ids")
        + candidate_categories.pop("_missing_retrieval_question_ids")
    )
    per_category, errors = compare_categories(baseline_categories, candidate_categories, args)
    if missing:
        errors.append(f"missing retrieval rows: {len(missing)}")

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.category_regression_gate.v1",
        "status": "passed" if not errors else "failed",
        "questions_file": str(args.questions_file),
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "top_k": args.top_k,
        "thresholds": {
            "max_category_recall_regression_pct": args.max_category_recall_regression_pct,
            "max_category_precision_regression_pct": args.max_category_precision_regression_pct,
            "max_category_invalid_extra_regression": args.max_category_invalid_extra_regression,
            "max_category_mrr_regression": args.max_category_mrr_regression,
            "max_category_ndcg_regression": args.max_category_ndcg_regression,
        },
        "baseline_metrics": baseline_metrics,
        "candidate_metrics": candidate_metrics,
        "per_category": per_category,
        "missing_retrieval_question_ids": missing,
        "errors": errors,
    }
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--max-category-recall-regression-pct", type=float, default=0.0)
    parser.add_argument("--max-category-precision-regression-pct", type=float, default=5.0)
    parser.add_argument("--max-category-invalid-extra-regression", type=float, default=0.5)
    parser.add_argument("--max-category-mrr-regression", type=float, default=0.05)
    parser.add_argument("--max-category-ndcg-regression", type=float, default=0.05)
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
                "category_count": len(QUESTION_TYPES),
                "errors": len(report["errors"]),
                "output": str(args.report),
            },
            sort_keys=True,
        )
    )
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
