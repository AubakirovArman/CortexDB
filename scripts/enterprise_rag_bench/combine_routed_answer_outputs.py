#!/usr/bin/env python3
"""Combine EnterpriseRAG answer artifacts with a deterministic routing policy."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


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


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def aggregate_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    recall_values = [
        float(row["document_recall_pct"])
        for row in rows
        if row.get("document_recall_pct") is not None
    ]
    invalid_values = [
        float(row["invalid_extra_docs"])
        for row in rows
        if row.get("invalid_extra_docs") is not None
    ]
    correct_pct = sum(1 for row in rows if row.get("answer_correct")) / len(rows) * 100.0 if rows else 0.0
    completeness = mean([float(row.get("completeness_pct") or 0.0) for row in rows])
    return {
        "average_correctness_pct": round(correct_pct, 2),
        "average_completeness_pct": completeness,
        "combined_correctness_completeness_score": round(correct_pct * completeness / 100.0, 2),
        "average_recall_pct": mean(recall_values),
        "average_invalid_extra_docs": mean(invalid_values),
        "total_questions": len(rows),
        "completed_questions": len(rows),
        "skipped_rows": 0,
        "num_corrected_questions": 0,
    }


def question_type_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        grouped.setdefault(str(row.get("question_type") or "unknown"), []).append(row)
    return {name: aggregate_stats(items) | {"count": len(items)} for name, items in grouped.items()}


def route_source(question_type: str | None, routed_types: set[str]) -> str:
    return "routed" if str(question_type or "") in routed_types else "default"


def require_same_ids(left: dict[str, Any], right: dict[str, Any], label: str) -> None:
    left_ids = set(left)
    right_ids = set(right)
    if left_ids != right_ids:
        missing = sorted(left_ids ^ right_ids)[:10]
        raise ValueError(f"{label} question_id mismatch: {missing}")


def build_combined(args: argparse.Namespace) -> dict[str, Any]:
    routed_types = {item.strip() for item in args.routed_question_types.split(",") if item.strip()}
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    default_answers = rows_by_id(read_jsonl(args.default_answers_file), "default answers")
    routed_answers = rows_by_id(read_jsonl(args.routed_answers_file), "routed answers")
    default_metrics = read_json(args.default_metrics_file)
    routed_metrics = read_json(args.routed_metrics_file)
    default_metric_rows = rows_by_id(default_metrics.get("questions", []), "default metrics")
    routed_metric_rows = rows_by_id(routed_metrics.get("questions", []), "routed metrics")

    for label, rows in [
        ("default answers", default_answers),
        ("routed answers", routed_answers),
        ("default metrics", default_metric_rows),
        ("routed metrics", routed_metric_rows),
    ]:
        require_same_ids(questions, rows, label)

    answer_rows: list[dict[str, Any]] = []
    metric_rows: list[dict[str, Any]] = []
    route_counts = {"default": 0, "routed": 0}

    for qid in questions:
        question_type = questions[qid].get("question_type")
        source = route_source(str(question_type), routed_types)
        route_counts[source] += 1
        answer = dict(routed_answers[qid] if source == "routed" else default_answers[qid])
        metric = dict(routed_metric_rows[qid] if source == "routed" else default_metric_rows[qid])
        route = {
            "policy": args.policy_name,
            "source": source,
            "question_type": question_type,
        }
        answer["route"] = route
        metric["route"] = route
        answer_rows.append(answer)
        metric_rows.append(metric)

    write_jsonl(args.output_answers_file, answer_rows)
    metrics = {
        "schema_version": "cortexdb.enterprise_rag_bench.routed_judge_metrics.v1",
        "policy_name": args.policy_name,
        "routed_question_types": sorted(routed_types),
        "default_answers_file": str(args.default_answers_file),
        "routed_answers_file": str(args.routed_answers_file),
        "default_metrics_file": str(args.default_metrics_file),
        "routed_metrics_file": str(args.routed_metrics_file),
        "answers_file": str(args.output_answers_file),
        "questions_file": str(args.questions_file),
        "judge_provider": "reused",
        "judge_model": {
            "default": default_metrics.get("judge_model"),
            "routed": routed_metrics.get("judge_model"),
        },
        "route_counts": route_counts,
        "questions": metric_rows,
        "aggregate_stats": aggregate_stats(metric_rows),
        "question_type_stats": question_type_stats(metric_rows),
        "source_token_totals": {
            "default": {
                "prompt_tokens": default_metrics.get("prompt_tokens", 0),
                "completion_tokens": default_metrics.get("completion_tokens", 0),
                "total_tokens": default_metrics.get("total_tokens", 0),
            },
            "routed": {
                "prompt_tokens": routed_metrics.get("prompt_tokens", 0),
                "completion_tokens": routed_metrics.get("completion_tokens", 0),
                "total_tokens": routed_metrics.get("total_tokens", 0),
            },
        },
    }
    write_json(args.output_metrics_file, metrics)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.routed_answer_report.v1",
        "policy_name": args.policy_name,
        "route_counts": route_counts,
        "answers_file": str(args.output_answers_file),
        "metrics_file": str(args.output_metrics_file),
        "aggregate_stats": metrics["aggregate_stats"],
        "note": "This combines already generated and already judged artifacts; it does not call an LLM.",
    }
    write_json(args.output_report_file, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--default-answers-file", type=Path, required=True)
    parser.add_argument("--default-metrics-file", type=Path, required=True)
    parser.add_argument("--routed-answers-file", type=Path, required=True)
    parser.add_argument("--routed-metrics-file", type=Path, required=True)
    parser.add_argument("--output-answers-file", type=Path, required=True)
    parser.add_argument("--output-metrics-file", type=Path, required=True)
    parser.add_argument("--output-report-file", type=Path, required=True)
    parser.add_argument("--policy-name", default="v7_selective_lexical_anchor")
    parser.add_argument(
        "--routed-question-types",
        default="basic,completeness,conflicting_info,constrained,project_related",
    )
    return parser.parse_args()


def main() -> int:
    print(json.dumps(build_combined(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
