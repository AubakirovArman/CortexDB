#!/usr/bin/env python3
"""Gate EnterpriseRAG retrieval quality from gold questions and clean retrieval."""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
from typing import Any

from official_clean import assert_clean_retrieval, read_jsonl, write_json
from progress_logging import ProgressLogger


LOGGER = ProgressLogger("retrieval-quality-gate")


def log(message: str) -> None:
    LOGGER.log(message)


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
    return [str(item) for item in row.get("document_ids", []) if str(item)][:limit]


def expected_doc_ids(question: dict[str, Any]) -> list[str]:
    return [str(item) for item in question.get("expected_doc_ids", []) if str(item)]


def round2(value: float) -> float:
    return round(value, 2)


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


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    log(f"loading questions {args.questions_file}")
    LOGGER.status(
        stage="retrieval_quality_gate",
        state="running",
        step=1,
        total_steps=4,
        questions_file=str(args.questions_file),
        retrieval_file=str(args.retrieval_file),
        top_k=args.top_k,
    )
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    log(f"loading retrieval {args.retrieval_file}")
    retrieval_rows = read_jsonl(args.retrieval_file)
    assert_clean_retrieval(retrieval_rows)
    retrieval = rows_by_id(retrieval_rows, "retrieval")
    LOGGER.status(
        stage="retrieval_quality_gate",
        state="running",
        step=2,
        total_steps=4,
        question_rows=len(questions),
        retrieval_rows=len(retrieval),
    )

    details: list[dict[str, Any]] = []
    recall_values: list[float] = []
    mrr_values: list[float] = []
    ndcg_values: list[float] = []
    invalid_values: list[float] = []
    per_type_values: dict[str, dict[str, list[float] | int]] = defaultdict(
        lambda: {
            "recall": [],
            "mrr": [],
            "ndcg": [],
            "invalid": [],
            "hit_questions": 0,
            "full_recall_questions": 0,
        }
    )

    missing_retrieval_rows: list[str] = []
    total_questions = len(questions)
    progress_every = max(1, getattr(args, "progress_every", 100))
    for processed, (qid, question) in enumerate(sorted(questions.items()), 1):
        expected_list = expected_doc_ids(question)
        if not expected_list:
            if processed % progress_every == 0 or processed == total_questions:
                LOGGER.progress(
                    stage="retrieval_quality_gate",
                    completed=processed,
                    total=total_questions,
                    unit="questions",
                    evaluated_questions=len(details),
                )
            continue
        expected = set(expected_list)
        retrieved = doc_ids(retrieval.get(qid), args.top_k)
        if qid not in retrieval:
            missing_retrieval_rows.append(qid)

        hits = len(expected & set(retrieved))
        recall = hits / len(expected) * 100.0
        invalid = len([doc_id for doc_id in retrieved if doc_id not in expected])
        mrr = reciprocal_rank(expected, retrieved)
        ndcg_value = ndcg(expected, retrieved)
        qtype = str(question.get("question_type") or "unknown")

        recall_values.append(recall)
        invalid_values.append(float(invalid))
        mrr_values.append(mrr)
        ndcg_values.append(ndcg_value)
        per_type_values[qtype]["recall"].append(recall)  # type: ignore[index,union-attr]
        per_type_values[qtype]["invalid"].append(float(invalid))  # type: ignore[index,union-attr]
        per_type_values[qtype]["mrr"].append(mrr)  # type: ignore[index,union-attr]
        per_type_values[qtype]["ndcg"].append(ndcg_value)  # type: ignore[index,union-attr]
        if hits > 0:
            per_type_values[qtype]["hit_questions"] = int(per_type_values[qtype]["hit_questions"]) + 1
        if hits == len(expected):
            per_type_values[qtype]["full_recall_questions"] = (
                int(per_type_values[qtype]["full_recall_questions"]) + 1
            )

        details.append(
            {
                "question_id": qid,
                "question_type": qtype,
                "expected_doc_ids": expected_list,
                "retrieved_doc_ids": retrieved,
                "recall_pct": round2(recall),
                "mrr": round2(mrr),
                "ndcg": round2(ndcg_value),
                "invalid_extra_docs": invalid,
            }
        )
        if processed % progress_every == 0 or processed == total_questions:
            LOGGER.progress(
                stage="retrieval_quality_gate",
                completed=processed,
                total=total_questions,
                unit="questions",
                evaluated_questions=len(details),
                hit_questions=sum(1 for value in recall_values if value > 0.0),
            )

    evaluated_questions = len(details)
    full_recall = sum(1 for value in recall_values if value == 100.0)
    hit_questions = sum(1 for value in recall_values if value > 0.0)
    metrics = {
        "evaluated_questions": evaluated_questions,
        "average_recall_pct": round2(mean(recall_values)),
        "hit_questions": hit_questions,
        "full_recall_questions": full_recall,
        "average_invalid_extra_docs": round2(mean(invalid_values)),
        "mrr": round2(mean(mrr_values)),
        "ndcg": round2(mean(ndcg_values)),
    }

    per_type: dict[str, Any] = {}
    for qtype, values in sorted(per_type_values.items()):
        recall = values["recall"]  # type: ignore[assignment]
        invalid = values["invalid"]  # type: ignore[assignment]
        mrr = values["mrr"]  # type: ignore[assignment]
        ndcg_vals = values["ndcg"]  # type: ignore[assignment]
        per_type[qtype] = {
            "questions": len(recall),  # type: ignore[arg-type]
            "average_recall_pct": round2(mean(recall)),  # type: ignore[arg-type]
            "hit_questions": int(values["hit_questions"]),
            "full_recall_questions": int(values["full_recall_questions"]),
            "average_invalid_extra_docs": round2(mean(invalid)),  # type: ignore[arg-type]
            "mrr": round2(mean(mrr)),  # type: ignore[arg-type]
            "ndcg": round2(mean(ndcg_vals)),  # type: ignore[arg-type]
        }

    errors: list[str] = []
    if missing_retrieval_rows:
        errors.append(f"missing retrieval rows: {len(missing_retrieval_rows)}")
    if metrics["average_recall_pct"] < args.min_average_recall_pct:
        errors.append(
            "average_recall_pct "
            f"{metrics['average_recall_pct']} < {args.min_average_recall_pct}"
        )
    if metrics["hit_questions"] < args.min_hit_questions:
        errors.append(f"hit_questions {metrics['hit_questions']} < {args.min_hit_questions}")
    if metrics["full_recall_questions"] < args.min_full_recall_questions:
        errors.append(
            "full_recall_questions "
            f"{metrics['full_recall_questions']} < {args.min_full_recall_questions}"
        )
    if metrics["average_invalid_extra_docs"] > args.max_average_invalid_extra_docs:
        errors.append(
            "average_invalid_extra_docs "
            f"{metrics['average_invalid_extra_docs']} > {args.max_average_invalid_extra_docs}"
        )
    if metrics["mrr"] < args.min_mrr:
        errors.append(f"mrr {metrics['mrr']} < {args.min_mrr}")
    if metrics["ndcg"] < args.min_ndcg:
        errors.append(f"ndcg {metrics['ndcg']} < {args.min_ndcg}")

    status = "passed" if not errors else "failed"
    log(
        "retrieval quality metrics "
        f"status={status} evaluated={evaluated_questions} "
        f"recall={metrics['average_recall_pct']} hit={metrics['hit_questions']} "
        f"full={metrics['full_recall_questions']} invalid={metrics['average_invalid_extra_docs']}"
    )
    LOGGER.status(
        stage="retrieval_quality_gate",
        state="running",
        step=3,
        total_steps=4,
        evaluated_questions=evaluated_questions,
        average_recall_pct=metrics["average_recall_pct"],
        hit_questions=metrics["hit_questions"],
        full_recall_questions=metrics["full_recall_questions"],
        average_invalid_extra_docs=metrics["average_invalid_extra_docs"],
        errors=len(errors),
    )

    return {
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_quality_gate.v1",
        "status": status,
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "top_k": args.top_k,
        "thresholds": {
            "min_average_recall_pct": args.min_average_recall_pct,
            "min_hit_questions": args.min_hit_questions,
            "min_full_recall_questions": args.min_full_recall_questions,
            "max_average_invalid_extra_docs": args.max_average_invalid_extra_docs,
            "min_mrr": args.min_mrr,
            "min_ndcg": args.min_ndcg,
        },
        "metrics": metrics,
        "per_type": per_type,
        "missing_retrieval_question_ids": missing_retrieval_rows,
        "errors": errors,
        "details": details if args.include_details else [],
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    metrics = report["metrics"]
    lines = [
        "# EnterpriseRAG Retrieval Quality Gate",
        "",
        f"- status: `{report['status']}`",
        f"- questions: `{report['questions_file']}`",
        f"- retrieval: `{report['retrieval_file']}`",
        f"- top_k: `{report['top_k']}`",
        "",
        "## Metrics",
        "",
        "| Metric | Value |",
        "| --- | ---: |",
    ]
    for key, value in metrics.items():
        lines.append(f"| `{key}` | {value} |")
    lines.extend(["", "## Per Type", "", "| Type | Questions | Recall | Hit | Full | Invalid | MRR | nDCG |", "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"])
    for qtype, values in sorted(report["per_type"].items()):
        lines.append(
            f"| `{qtype}` | {values['questions']} | {values['average_recall_pct']} | "
            f"{values['hit_questions']} | {values['full_recall_questions']} | "
            f"{values['average_invalid_extra_docs']} | {values['mrr']} | {values['ndcg']} |"
        )
    if report["errors"]:
        lines.extend(["", "## Errors", ""])
        lines.extend(f"- {error}" for error in report["errors"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--min-average-recall-pct", type=float, default=0.0)
    parser.add_argument("--min-hit-questions", type=int, default=0)
    parser.add_argument("--min-full-recall-questions", type=int, default=0)
    parser.add_argument("--max-average-invalid-extra-docs", type=float, default=10.0)
    parser.add_argument("--min-mrr", type=float, default=0.0)
    parser.add_argument("--min-ndcg", type=float, default=0.0)
    parser.add_argument("--include-details", action="store_true")
    parser.add_argument("--progress-every", type=int, default=100)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    global LOGGER
    LOGGER = ProgressLogger(
        "retrieval-quality-gate",
        log_file=args.log_file,
        status_file=args.status_file,
    )
    if args.top_k <= 0:
        parser.error("--top-k must be positive")
    if args.progress_every <= 0:
        parser.error("--progress-every must be positive")
    report = build_report(args)
    write_json(args.report, report)
    log(f"wrote retrieval quality report {args.report}")
    if args.markdown:
        write_markdown(args.markdown, report)
        log(f"wrote retrieval quality markdown {args.markdown}")
    LOGGER.status(
        stage="retrieval_quality_gate",
        state=report["status"],
        step=4,
        total_steps=4,
        report=str(args.report),
        markdown=str(args.markdown) if args.markdown else None,
        errors=len(report["errors"]),
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        LOGGER.status(stage="retrieval_quality_gate", state="failed", error=str(error))
        raise
