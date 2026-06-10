#!/usr/bin/env python3
"""Compare two EnterpriseRAG retrieval runs question-by-question."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Retrieval Run Comparison",
        "",
        f"- status: `{report['status']}`",
        f"- baseline: `{report['baseline_retrieval_file']}`",
        f"- candidate: `{report['candidate_retrieval_file']}`",
        f"- questions: `{report['questions']}`",
        "",
        "## Summary",
        "",
        "| Metric | Baseline | Candidate | Delta |",
        "| --- | ---: | ---: | ---: |",
    ]
    for key in ("average_recall_pct", "full_recall_questions", "hit_questions"):
        metric = report["metrics"][key]
        lines.append(f"| `{key}` | {metric['baseline']} | {metric['candidate']} | {metric['delta']} |")
    lines.extend(
        [
            "",
            f"- improved_questions: `{len(report['improved_question_ids'])}`",
            f"- regressed_questions: `{len(report['regressed_question_ids'])}`",
            f"- unchanged_questions: `{len(report['unchanged_question_ids'])}`",
            "",
            "## Per Type",
            "",
            "| Type | Baseline Recall | Candidate Recall | Delta | Improved | Regressed |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for qtype, stats in sorted(report["per_type"].items()):
        lines.append(
            f"| `{qtype}` | {stats['baseline_average_recall_pct']} | "
            f"{stats['candidate_average_recall_pct']} | {stats['delta_average_recall_pct']} | "
            f"{stats['improved_questions']} | {stats['regressed_questions']} |"
        )
    if report["errors"]:
        lines.extend(["", "## Errors", ""])
        lines.extend(f"- {error}" for error in report["errors"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        values[qid] = row
    return values


def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def recall_pct(question: dict[str, Any], row: dict[str, Any] | None, limit: int) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    docs = set(doc_ids(row)[:limit])
    return round(len(expected & docs) / len(expected) * 100.0, 2)


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def metric_block(baseline: float | int, candidate: float | int) -> dict[str, float | int]:
    delta = candidate - baseline
    if isinstance(baseline, int) and isinstance(candidate, int):
        return {"baseline": baseline, "candidate": candidate, "delta": int(delta)}
    return {"baseline": baseline, "candidate": candidate, "delta": round(float(delta), 2)}


def build_gate_errors(args: argparse.Namespace, metrics: dict[str, Any], regressed: list[str]) -> list[str]:
    errors: list[str] = []
    average_delta = float(metrics["average_recall_pct"]["delta"])
    full_delta = int(metrics["full_recall_questions"]["delta"])
    hit_delta = int(metrics["hit_questions"]["delta"])
    if args.min_average_recall_delta_pct is not None and average_delta < args.min_average_recall_delta_pct:
        errors.append(
            "average_recall_delta_pct "
            f"{average_delta} < {args.min_average_recall_delta_pct}"
        )
    if args.min_full_recall_delta is not None and full_delta < args.min_full_recall_delta:
        errors.append(f"full_recall_delta {full_delta} < {args.min_full_recall_delta}")
    if args.min_hit_delta is not None and hit_delta < args.min_hit_delta:
        errors.append(f"hit_delta {hit_delta} < {args.min_hit_delta}")
    if args.max_regressed_questions is not None and len(regressed) > args.max_regressed_questions:
        errors.append(
            "regressed_questions "
            f"{len(regressed)} > {args.max_regressed_questions}: {regressed}"
        )
    return errors


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    candidate = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidate")

    details: list[dict[str, Any]] = []
    baseline_values: list[float] = []
    candidate_values: list[float] = []
    improved: list[str] = []
    regressed: list[str] = []
    unchanged: list[str] = []
    per_type_values: dict[str, dict[str, list[float] | int]] = defaultdict(
        lambda: {"baseline": [], "candidate": [], "improved": 0, "regressed": 0}
    )

    for qid, question in sorted(questions.items()):
        before = recall_pct(question, baseline.get(qid), args.limit)
        after = recall_pct(question, candidate.get(qid), args.limit)
        if before is None or after is None:
            continue
        qtype = str(question.get("question_type") or "unknown")
        baseline_values.append(before)
        candidate_values.append(after)
        per_type_values[qtype]["baseline"].append(before)  # type: ignore[index,union-attr]
        per_type_values[qtype]["candidate"].append(after)  # type: ignore[index,union-attr]
        delta = round(after - before, 2)
        if delta > 0:
            improved.append(qid)
            per_type_values[qtype]["improved"] = int(per_type_values[qtype]["improved"]) + 1
        elif delta < 0:
            regressed.append(qid)
            per_type_values[qtype]["regressed"] = int(per_type_values[qtype]["regressed"]) + 1
        else:
            unchanged.append(qid)
        details.append(
            {
                "question_id": qid,
                "question_type": qtype,
                "baseline_recall_pct": before,
                "candidate_recall_pct": after,
                "delta_recall_pct": delta,
                "baseline_doc_ids": doc_ids(baseline.get(qid))[: args.limit],
                "candidate_doc_ids": doc_ids(candidate.get(qid))[: args.limit],
                "expected_doc_ids": question.get("expected_doc_ids", []),
            }
        )

    baseline_full = sum(1 for value in baseline_values if value == 100.0)
    candidate_full = sum(1 for value in candidate_values if value == 100.0)
    baseline_hits = sum(1 for value in baseline_values if value > 0)
    candidate_hits = sum(1 for value in candidate_values if value > 0)
    per_type: dict[str, Any] = {}
    for qtype, values in sorted(per_type_values.items()):
        before_values = values["baseline"]  # type: ignore[assignment]
        after_values = values["candidate"]  # type: ignore[assignment]
        before_avg = mean(before_values)  # type: ignore[arg-type]
        after_avg = mean(after_values)  # type: ignore[arg-type]
        per_type[qtype] = {
            "baseline_average_recall_pct": before_avg,
            "candidate_average_recall_pct": after_avg,
            "delta_average_recall_pct": round(after_avg - before_avg, 2),
            "improved_questions": int(values["improved"]),
            "regressed_questions": int(values["regressed"]),
        }

    metrics = {
        "average_recall_pct": metric_block(mean(baseline_values), mean(candidate_values)),
        "full_recall_questions": metric_block(baseline_full, candidate_full),
        "hit_questions": metric_block(baseline_hits, candidate_hits),
    }
    errors = build_gate_errors(args, metrics, regressed)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_comparison.v1",
        "status": "passed" if not errors else "failed",
        "questions_file": str(args.questions_file),
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "details_file": str(args.output_jsonl),
        "questions": len(details),
        "limit": args.limit,
        "metrics": metrics,
        "thresholds": {
            "min_average_recall_delta_pct": args.min_average_recall_delta_pct,
            "min_full_recall_delta": args.min_full_recall_delta,
            "min_hit_delta": args.min_hit_delta,
            "max_regressed_questions": args.max_regressed_questions,
        },
        "improved_question_ids": improved,
        "regressed_question_ids": regressed,
        "unchanged_question_ids": unchanged,
        "per_type": per_type,
        "errors": errors,
    }
    write_jsonl(args.output_jsonl, details)
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--min-average-recall-delta-pct", type=float)
    parser.add_argument("--min-full-recall-delta", type=int)
    parser.add_argument("--min-hit-delta", type=int)
    parser.add_argument("--max-regressed-questions", type=int)
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be positive")
    if args.max_regressed_questions is not None and args.max_regressed_questions < 0:
        parser.error("--max-regressed-questions must be non-negative")
    return args


def main() -> int:
    report = run(parse_args())
    print(json.dumps(report, sort_keys=True))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
