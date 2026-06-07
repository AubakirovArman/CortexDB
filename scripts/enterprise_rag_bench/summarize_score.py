#!/usr/bin/env python3
"""Write a compact EnterpriseRAG-Bench score summary."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


FIELD_MAP = {
    "Overall Score": "combined_correctness_completeness_score",
    "Answer Correctness": "average_correctness_pct",
    "Answer Completeness": "average_completeness_pct",
    "Document Recall": "average_recall_pct",
    "Invalid Extra Docs": "average_invalid_extra_docs",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl_count(path: Path | None) -> int | None:
    if path is None or not path.exists():
        return None
    return sum(1 for line in path.read_text(encoding="utf-8").splitlines() if line)


def read_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def token_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    token_rows = [
        {
            "question_id": str(row.get("question_id")),
            "prompt_tokens": int(row.get("prompt_tokens", 0) or 0),
            "completion_tokens": int(row.get("completion_tokens", 0) or 0),
            "total_tokens": int(row.get("total_tokens", 0) or 0),
        }
        for row in rows
        if row.get("total_tokens") is not None
    ]
    token_rows = [row for row in token_rows if row["total_tokens"] > 0]
    total = sum(row["total_tokens"] for row in token_rows)
    count = len(token_rows)
    return {
        "per_question_available": count > 0,
        "questions_with_usage": count,
        "prompt_tokens": sum(row["prompt_tokens"] for row in token_rows),
        "completion_tokens": sum(row["completion_tokens"] for row in token_rows),
        "total_tokens": total,
        "avg_total_tokens_per_question": round(total / count, 2) if count else None,
        "max_total_tokens_question": max(token_rows, key=lambda row: row["total_tokens"]) if token_rows else None,
        "top_total_tokens_questions": sorted(token_rows, key=lambda row: row["total_tokens"], reverse=True)[:10],
    }


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_markdown(path: Path, payload: dict[str, Any]) -> None:
    tokens = payload["token_accounting"]
    lines = [
        "# EnterpriseRAG-Bench Score Summary",
        "",
        f"- Run label: `{payload['run_label']}`",
        f"- Questions: `{payload['questions']}`",
        f"- Metrics file: `{payload['metrics_file']}`",
        "",
        "| Metric | Value |",
        "| --- | ---: |",
    ]
    for key, value in payload["scorecard"].items():
        lines.append(f"| {key} | {value} |")
    lines.extend(
        [
            "",
            "## Token Accounting",
            "",
            f"- Generation per-question usage: `{tokens['generation']['per_question_available']}`",
            f"- Judge per-question usage: `{tokens['judge']['per_question_available']}`",
            f"- Generation total tokens: `{tokens['generation']['total_tokens']}`",
            f"- Judge total tokens: `{tokens['judge']['total_tokens']}`",
            f"- Judge avg tokens/question: `{tokens['judge']['avg_total_tokens_per_question']}`",
        ]
    )
    top_judge_questions = tokens["judge"]["top_total_tokens_questions"]
    if top_judge_questions:
        lines.extend(["", "### Top Judge Token Questions", "", "| Question | Prompt | Completion | Total |", "| --- | ---: | ---: | ---: |"])
        for row in top_judge_questions:
            lines.append(
                f"| `{row['question_id']}` | {row['prompt_tokens']} | {row['completion_tokens']} | {row['total_tokens']} |"
            )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def summarize(args: argparse.Namespace) -> dict[str, Any]:
    metrics = read_json(args.metrics_file)
    aggregate = metrics.get("aggregate_stats", {})
    scorecard = {label: aggregate.get(source) for label, source in FIELD_MAP.items()}
    questions = aggregate.get("total_questions") or read_jsonl_count(args.questions_file)
    answer_rows = read_jsonl(args.answers_file)
    metric_rows = metrics.get("questions", [])
    payload = {
        "schema_version": "cortexdb.enterprise_rag_bench.score_summary.v1",
        "run_label": args.run_label,
        "questions": questions,
        "metrics_file": str(args.metrics_file),
        "answers_file": str(args.answers_file) if args.answers_file else None,
        "questions_file": str(args.questions_file) if args.questions_file else None,
        "scorecard": scorecard,
        "aggregate_stats": aggregate,
        "question_type_stats": metrics.get("question_type_stats", {}),
        "token_accounting": {
            "generation": token_stats(answer_rows),
            "judge": token_stats(metric_rows if isinstance(metric_rows, list) else []),
        },
    }
    write_json(args.output, payload)
    if args.markdown:
        write_markdown(args.markdown, payload)
    return payload


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metrics-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--answers-file", type=Path)
    parser.add_argument("--questions-file", type=Path)
    parser.add_argument("--run-label", default="enterprise-rag-bench")
    return parser.parse_args()


def main() -> int:
    print(json.dumps(summarize(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
