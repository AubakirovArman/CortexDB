#!/usr/bin/env python3
"""Analyze CortexDB LongMemEval v1 official run errors."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


KS = (1, 3, 5, 10)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        require(isinstance(row, dict), f"{path}:{line_number}: expected object")
        rows.append(row)
    return rows


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def newest_file(directory: Path, pattern: str) -> Path:
    candidates = [path for path in directory.glob(pattern) if path.is_file()]
    require(candidates, f"no files matching {pattern} in {directory}")
    return max(candidates, key=lambda path: path.stat().st_mtime)


def load_retrieval_rows(path: Path) -> dict[str, dict[str, Any]]:
    rows: dict[str, dict[str, Any]] = {}
    with path.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            question_id = str(row["question_id"])
            ranked_ids = [
                str(item.get("corpus_id", ""))
                for item in row["retrieval_results"].get("ranked_items", [])
            ]
            rows[question_id] = {
                "question_id": question_id,
                "question_type": row["question_type"],
                "question": row["question"],
                "answer": row["answer"],
                "answer_session_ids": [str(value) for value in row.get("answer_session_ids", [])],
                "aggregate_skip_reason": row.get("aggregate_skip_reason", ""),
                "ranked_ids": ranked_ids,
                "metrics": row["retrieval_results"].get("metrics", {}),
            }
            if line_number % 100 == 0:
                print(f"loaded retrieval rows: {line_number}")
    return rows


def answer_rank(ranked_ids: list[str], answer_ids: list[str]) -> int | None:
    answer_set = set(answer_ids)
    for index, corpus_id in enumerate(ranked_ids, start=1):
        if corpus_id in answer_set or ("answer" in corpus_id and "noans" not in corpus_id):
            return index
    return None


def metric(metrics: dict[str, Any], name: str) -> float:
    value = metrics.get("session", {}).get(name)
    return float(value) if isinstance(value, (int, float)) else 0.0


def read_official_metrics(path: Path | None) -> dict[str, float]:
    if path is None or not path.exists():
        return {}
    text = path.read_text(encoding="utf-8")
    values: dict[str, float] = {}
    for name, value in re.findall(r"([a-z_]+@\d+)\s*=\s*([0-9.]+)", text):
        values[name] = float(value)
    return values


def classify_error(row: dict[str, Any], retrieval: dict[str, Any]) -> str:
    if "_abs" in row["question_id"]:
        return "abstention_failure"
    if metric(retrieval["metrics"], "recall_any@10") < 1.0:
        return "retrieval_miss_no_answer_session_top10"
    if metric(retrieval["metrics"], "recall_all@10") < 1.0:
        return "retrieval_partial_miss_top10"
    rank = retrieval["answer_rank"]
    if rank is not None and rank > 5:
        return "evidence_low_rank_reader_failure"
    question_type = retrieval["question_type"]
    if question_type == "single-session-preference":
        return "preference_reader_failure"
    if question_type == "multi-session":
        return "multi_session_reader_failure"
    if question_type == "temporal-reasoning":
        return "temporal_reasoning_failure"
    if question_type == "knowledge-update":
        return "knowledge_update_reader_failure"
    return "reader_or_prompt_failure"


def summarize_numeric(values: list[int | None]) -> dict[str, float | int | None]:
    present = [value for value in values if value is not None]
    if not present:
        return {"count": 0, "min": None, "max": None, "mean": None}
    return {
        "count": len(present),
        "min": min(present),
        "max": max(present),
        "mean": sum(present) / len(present),
    }


def build_report(
    retrieval_rows: dict[str, dict[str, Any]],
    eval_rows: list[dict[str, Any]],
    official_metrics: dict[str, float],
) -> tuple[dict[str, Any], list[dict[str, Any]], list[dict[str, Any]]]:
    diagnostics: list[dict[str, Any]] = []
    false_cases: list[dict[str, Any]] = []
    by_type: dict[str, list[int]] = defaultdict(list)
    by_category = Counter()
    ranks_by_type: dict[str, list[int | None]] = defaultdict(list)
    metric_sums: dict[str, float] = defaultdict(float)

    for eval_row in eval_rows:
        question_id = str(eval_row["question_id"])
        retrieval = retrieval_rows[question_id]
        rank = answer_rank(retrieval["ranked_ids"], retrieval["answer_session_ids"])
        retrieval["answer_rank"] = rank
        label = bool(eval_row["autoeval_label"]["label"])
        question_type = retrieval["question_type"]
        by_type[question_type].append(1 if label else 0)
        ranks_by_type[question_type].append(rank)
        for k in KS:
            metric_sums[f"recall_all@{k}"] += metric(retrieval["metrics"], f"recall_all@{k}")
            metric_sums[f"ndcg_any@{k}"] += metric(retrieval["metrics"], f"ndcg_any@{k}")

        diagnostic = {
            "question_id": question_id,
            "question_type": question_type,
            "correct": label,
            "answer_rank": rank,
            "answer_in_top1": rank is not None and rank <= 1,
            "answer_in_top3": rank is not None and rank <= 3,
            "answer_in_top5": rank is not None and rank <= 5,
            "answer_in_top10": rank is not None and rank <= 10,
            "recall_all_at_10": metric(retrieval["metrics"], "recall_all@10"),
            "ndcg_any_at_10": metric(retrieval["metrics"], "ndcg_any@10"),
            "ranked_ids": retrieval["ranked_ids"],
            "answer_session_ids": retrieval["answer_session_ids"],
        }
        diagnostics.append(diagnostic)
        if not label:
            category = classify_error(eval_row, retrieval)
            by_category[category] += 1
            false_cases.append(
                {
                    **diagnostic,
                    "error_category": category,
                    "question": retrieval["question"],
                    "answer": retrieval["answer"],
                    "hypothesis": eval_row["hypothesis"],
                }
            )

    total = len(eval_rows)
    correct = sum(sum(values) for values in by_type.values())
    report = {
        "schema_version": "cortexdb.longmemeval.v1.error_analysis.v1",
        "questions": total,
        "correct": correct,
        "false_count": total - correct,
        "accuracy": correct / total if total else 0.0,
        "by_question_type": {
            name: {
                "count": len(values),
                "correct": sum(values),
                "false": len(values) - sum(values),
                "accuracy": sum(values) / len(values),
                "answer_rank": summarize_numeric(ranks_by_type[name]),
            }
            for name, values in sorted(by_type.items())
        },
        "false_by_category": dict(sorted(by_category.items())),
        "official_retrieval_metrics": official_metrics,
        "diagnostic_retrieval_metric_averages": {
            name: value / total if total else 0.0
            for name, value in sorted(metric_sums.items())
        },
        "top_error_categories": by_category.most_common(),
    }
    return report, diagnostics, false_cases


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# LongMemEval v1 Error Analysis",
        "",
        f"- questions: `{report['questions']}`",
        f"- correct: `{report['correct']}`",
        f"- false: `{report['false_count']}`",
        f"- accuracy: `{report['accuracy']:.4f}`",
        "",
        "## False Cases By Category",
        "",
        "| Category | Count |",
        "| --- | ---: |",
    ]
    for category, count in report["false_by_category"].items():
        lines.append(f"| `{category}` | `{count}` |")
    lines += ["", "## Accuracy By Question Type", "", "| Type | Accuracy | False | Answer-rank mean |", "| --- | ---: | ---: | ---: |"]
    for name, row in report["by_question_type"].items():
        mean_rank = row["answer_rank"]["mean"]
        mean_display = "" if mean_rank is None else f"{mean_rank:.2f}"
        lines.append(f"| `{name}` | `{row['accuracy']:.4f}` | `{row['false']}` | `{mean_display}` |")
    lines += ["", "## Official Retrieval Metrics", "", "| Metric | Value |", "| --- | ---: |"]
    for name, value in report["official_retrieval_metrics"].items():
        lines.append(f"| `{name}` | `{value:.4f}` |")
    lines += ["", "## Diagnostic Per-Row Metric Averages", "", "| Metric | Value |", "| --- | ---: |"]
    for name, value in report["diagnostic_retrieval_metric_averages"].items():
        lines.append(f"| `{name}` | `{value:.4f}` |")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-log", type=Path, default=Path("target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl"))
    parser.add_argument("--eval-results", type=Path)
    parser.add_argument("--generation-dir", type=Path, default=Path("target/longmemeval-v1/generation"))
    parser.add_argument("--official-metrics", type=Path, default=Path("target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt"))
    parser.add_argument("--output-root", type=Path, default=Path("target/longmemeval-v1/analysis"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    eval_results = args.eval_results or newest_file(args.generation_dir, "*.eval-results-gpt-4o")
    retrieval_rows = load_retrieval_rows(args.retrieval_log)
    eval_rows = read_jsonl(eval_results)
    require(len(retrieval_rows) == len(eval_rows), "retrieval/eval row counts differ")
    official_metrics = read_official_metrics(args.official_metrics)
    report, diagnostics, false_cases = build_report(retrieval_rows, eval_rows, official_metrics)
    args.output_root.mkdir(parents=True, exist_ok=True)
    write_json(args.output_root / "error_report.json", report)
    write_jsonl(args.output_root / "retrieval_diagnostics.jsonl", diagnostics)
    write_jsonl(args.output_root / "false_cases.jsonl", false_cases)
    write_markdown(args.output_root / "error_report.md", report)
    print(json.dumps({"output_root": str(args.output_root), **report}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
