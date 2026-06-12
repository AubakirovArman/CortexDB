#!/usr/bin/env python3
"""Compare EnterpriseRAG answer budget traces and optional judge reports."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def read_json(path: Path | None) -> dict[str, Any]:
    if path is None:
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: expected JSON object")
    return payload


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        payload = json.loads(line)
        if not isinstance(payload, dict):
            raise ValueError(f"{path}:{line_number}: expected JSON object")
        rows.append(payload)
    return rows


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def number(value: Any, default: float = 0.0) -> float:
    if isinstance(value, bool):
        return default
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value)
        except ValueError:
            return default
    return default


def int_number(value: Any, default: int = 0) -> int:
    return int(number(value, float(default)))


def derive_static_trace(
    rows: list[dict[str, Any]],
    *,
    top_k_context: int,
    max_chars_per_doc: int,
    max_tokens: int,
) -> list[dict[str, Any]]:
    derived: list[dict[str, Any]] = []
    for row in rows:
        retrieved = int_number(row.get("retrieved_doc_count"), 0)
        derived.append(
            {
                "question_id": row.get("question_id"),
                "answer_intent": "static",
                "answer_intent_score": 0,
                "context_mode": "static",
                "active_top_k_context": top_k_context,
                "selected_result_limit": top_k_context,
                "active_max_chars_per_doc": max_chars_per_doc,
                "active_max_tokens": max_tokens,
                "retrieved_doc_count": retrieved,
                "used_doc_count": min(retrieved, top_k_context),
                "adaptive_budget_applied": False,
                "high_level_override_applied": False,
                "budget_profile": None,
                "trace_source": "derived_static",
            }
        )
    return derived


def summarize_trace(rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {
            "questions": 0,
            "adaptive_budget_questions": 0,
            "high_level_override_questions": 0,
            "avg_selected_result_limit": 0.0,
            "avg_used_doc_count": 0.0,
            "avg_max_chars_per_doc": 0.0,
            "avg_max_tokens": 0.0,
            "max_selected_result_limit": 0,
            "max_max_tokens": 0,
            "by_answer_intent": {},
        }
    by_intent: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_intent[str(row.get("answer_intent") or "unknown")].append(row)

    def avg(key: str, items: list[dict[str, Any]] = rows) -> float:
        return round(sum(number(row.get(key)) for row in items) / len(items), 2)

    return {
        "questions": len(rows),
        "adaptive_budget_questions": sum(1 for row in rows if row.get("adaptive_budget_applied")),
        "high_level_override_questions": sum(
            1 for row in rows if row.get("high_level_override_applied")
        ),
        "avg_selected_result_limit": avg("selected_result_limit"),
        "avg_used_doc_count": avg("used_doc_count"),
        "avg_max_chars_per_doc": avg("active_max_chars_per_doc"),
        "avg_max_tokens": avg("active_max_tokens"),
        "max_selected_result_limit": max(int_number(row.get("selected_result_limit")) for row in rows),
        "max_max_tokens": max(int_number(row.get("active_max_tokens")) for row in rows),
        "by_answer_intent": {
            intent: {
                "questions": len(items),
                "avg_selected_result_limit": avg("selected_result_limit", items),
                "avg_used_doc_count": avg("used_doc_count", items),
                "avg_max_chars_per_doc": avg("active_max_chars_per_doc", items),
                "avg_max_tokens": avg("active_max_tokens", items),
            }
            for intent, items in sorted(by_intent.items())
        },
    }


def diff_summary(candidate: dict[str, Any], baseline: dict[str, Any]) -> dict[str, Any]:
    keys = [
        "adaptive_budget_questions",
        "high_level_override_questions",
        "avg_selected_result_limit",
        "avg_used_doc_count",
        "avg_max_chars_per_doc",
        "avg_max_tokens",
        "max_selected_result_limit",
        "max_max_tokens",
    ]
    return {key: round(number(candidate.get(key)) - number(baseline.get(key)), 2) for key in keys}


def extract_answer_metrics(report: dict[str, Any]) -> dict[str, Any]:
    if not report:
        return {}
    return {
        "questions": report.get("questions"),
        "prompt_tokens": report.get("prompt_tokens"),
        "completion_tokens": report.get("completion_tokens"),
        "total_tokens": report.get("total_tokens"),
        "budget_trace_questions": report.get("budget_trace_questions"),
        "adaptive_budget_questions": report.get("adaptive_budget_questions"),
        "model": report.get("model"),
    }


def extract_judge_metrics(report: dict[str, Any]) -> dict[str, Any]:
    if not report:
        return {}
    stats = report.get("aggregate_stats")
    if not isinstance(stats, dict):
        stats = report.get("judge_summary") if isinstance(report.get("judge_summary"), dict) else {}
    return {
        "overall": stats.get("combined_correctness_completeness_score")
        or report.get("overall"),
        "answer_correctness_pct": stats.get("average_correctness_pct")
        or report.get("answer_correctness_pct"),
        "answer_completeness_pct": stats.get("average_completeness_pct")
        or report.get("answer_completeness_pct"),
        "prompt_tokens": report.get("prompt_tokens"),
        "completion_tokens": report.get("completion_tokens"),
        "total_tokens": report.get("total_tokens"),
        "model": report.get("model") or report.get("judge_model"),
    }


def metric_delta(candidate: dict[str, Any], baseline: dict[str, Any]) -> dict[str, Any]:
    keys = sorted(set(candidate) | set(baseline))
    result: dict[str, Any] = {}
    for key in keys:
        before = baseline.get(key)
        after = candidate.get(key)
        if isinstance(before, (int, float)) or isinstance(after, (int, float)):
            result[key] = round(number(after) - number(before), 2)
    return result


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    candidate_rows = read_jsonl(args.candidate_trace)
    if args.baseline_trace:
        baseline_rows = read_jsonl(args.baseline_trace)
        baseline_mode = "trace"
    else:
        baseline_rows = derive_static_trace(
            candidate_rows,
            top_k_context=args.static_top_k_context,
            max_chars_per_doc=args.static_max_chars_per_doc,
            max_tokens=args.static_max_tokens,
        )
        baseline_mode = "derived_static"

    baseline_trace = summarize_trace(baseline_rows)
    candidate_trace = summarize_trace(candidate_rows)
    baseline_answer = extract_answer_metrics(read_json(args.baseline_answer_report))
    candidate_answer = extract_answer_metrics(read_json(args.candidate_answer_report))
    baseline_judge = extract_judge_metrics(read_json(args.baseline_judge_results))
    candidate_judge = extract_judge_metrics(read_json(args.candidate_judge_results))

    return {
        "schema_version": "cortexdb.enterprise_rag_bench.answer_budget_ab.v1",
        "baseline_mode": baseline_mode,
        "baseline_trace_file": str(args.baseline_trace) if args.baseline_trace else None,
        "candidate_trace_file": str(args.candidate_trace),
        "static_baseline": {
            "top_k_context": args.static_top_k_context,
            "max_chars_per_doc": args.static_max_chars_per_doc,
            "max_tokens": args.static_max_tokens,
        }
        if baseline_mode == "derived_static"
        else None,
        "baseline_trace": baseline_trace,
        "candidate_trace": candidate_trace,
        "trace_delta": diff_summary(candidate_trace, baseline_trace),
        "answer_metrics": {
            "baseline": baseline_answer,
            "candidate": candidate_answer,
            "delta": metric_delta(candidate_answer, baseline_answer),
        },
        "judge_metrics": {
            "baseline": baseline_judge,
            "candidate": candidate_judge,
            "delta": metric_delta(candidate_judge, baseline_judge),
        },
        "notes": [
            "Derived static baselines compare planned budget only; they do not estimate actual model tokens.",
            "Use answer/judge reports for official score and real token deltas when available.",
        ],
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    trace_delta = report["trace_delta"]
    judge_delta = report["judge_metrics"]["delta"]
    answer_delta = report["answer_metrics"]["delta"]
    lines = [
        "# EnterpriseRAG Answer Budget A/B",
        "",
        f"- baseline_mode: `{report['baseline_mode']}`",
        f"- candidate_trace: `{report['candidate_trace_file']}`",
        "",
        "## Trace Delta",
        "",
        "| Metric | Delta |",
        "| --- | ---: |",
    ]
    for key, value in trace_delta.items():
        lines.append(f"| `{key}` | {value} |")
    if answer_delta:
        lines.extend(["", "## Answer Token Delta", "", "| Metric | Delta |", "| --- | ---: |"])
        for key, value in answer_delta.items():
            lines.append(f"| `{key}` | {value} |")
    if judge_delta:
        lines.extend(["", "## Judge Metric Delta", "", "| Metric | Delta |", "| --- | ---: |"])
        for key, value in judge_delta.items():
            lines.append(f"| `{key}` | {value} |")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-trace", type=Path, required=True)
    parser.add_argument("--baseline-trace", type=Path)
    parser.add_argument("--candidate-answer-report", type=Path)
    parser.add_argument("--baseline-answer-report", type=Path)
    parser.add_argument("--candidate-judge-results", type=Path)
    parser.add_argument("--baseline-judge-results", type=Path)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--static-top-k-context", type=int, default=8)
    parser.add_argument("--static-max-chars-per-doc", type=int, default=2200)
    parser.add_argument("--static-max-tokens", type=int, default=420)
    args = parser.parse_args()
    if args.static_top_k_context <= 0:
        parser.error("--static-top-k-context must be positive")
    if args.static_max_chars_per_doc <= 0:
        parser.error("--static-max-chars-per-doc must be positive")
    if args.static_max_tokens <= 0:
        parser.error("--static-max-tokens must be positive")
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
                "baseline_mode": report["baseline_mode"],
                "candidate_questions": report["candidate_trace"]["questions"],
                "trace_delta": report["trace_delta"],
                "output": str(args.report),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
