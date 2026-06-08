#!/usr/bin/env python3
"""Inject deeper candidates for evidence-coverage question types.

This is a retrieval-only policy. It keeps the stable baseline prefix and uses a
small number of deeper candidate documents to improve full-evidence coverage for
question types such as completeness. It does not use answer text or LLM calls.
"""

from __future__ import annotations

import argparse
import json
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


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def route_types(value: str) -> set[str]:
    return {item.strip() for item in value.split(",") if item.strip()}


def merged_docs(
    *,
    baseline_docs: list[str],
    candidate_docs: list[str],
    keep_baseline: int,
    candidate_start: int,
    candidate_scan: int,
    limit: int,
) -> list[str]:
    selected: list[str] = []
    for doc_id in baseline_docs[:keep_baseline]:
        if doc_id not in selected:
            selected.append(doc_id)
    for doc_id in candidate_docs[candidate_start : candidate_start + candidate_scan]:
        if doc_id not in selected:
            selected.append(doc_id)
        if len(selected) >= limit:
            break
    for doc_id in baseline_docs:
        if doc_id not in selected:
            selected.append(doc_id)
        if len(selected) >= limit:
            break
    return selected[:limit]


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline_rows = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    candidate_rows = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidates")
    routed_types = route_types(args.routed_question_types)

    output_rows: list[dict[str, Any]] = []
    changed_rows = 0
    recall_values: list[float] = []
    per_type: dict[str, list[float]] = {}

    for qid, baseline in baseline_rows.items():
        question = questions.get(qid, baseline)
        qtype = str(question.get("question_type") or "unknown")
        baseline_docs = doc_ids(baseline)
        if qtype in routed_types:
            selected = merged_docs(
                baseline_docs=baseline_docs,
                candidate_docs=doc_ids(candidate_rows.get(qid)),
                keep_baseline=args.keep_baseline,
                candidate_start=args.candidate_start,
                candidate_scan=args.candidate_scan,
                limit=args.limit,
            )
        else:
            selected = baseline_docs[: args.limit]
        row = dict(baseline)
        if selected != baseline_docs[: args.limit]:
            changed_rows += 1
        row["document_ids"] = selected
        row["route"] = {
            "policy": args.policy_name,
            "question_type": qtype,
            "routed": qtype in routed_types,
            "keep_baseline": args.keep_baseline,
            "candidate_start": args.candidate_start,
            "candidate_scan": args.candidate_scan,
            "limit": args.limit,
        }
        output_rows.append(row)
        recall = recall_pct(question, selected)
        if recall is not None:
            recall_values.append(recall)
            per_type.setdefault(qtype, []).append(recall)

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.coverage_route.v1",
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "output": str(args.output),
        "policy_name": args.policy_name,
        "routed_question_types": sorted(routed_types),
        "keep_baseline": args.keep_baseline,
        "candidate_start": args.candidate_start,
        "candidate_scan": args.candidate_scan,
        "limit": args.limit,
        "changed_rows": changed_rows,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "per_type_recall_pct": {
            key: round(sum(values) / len(values), 2)
            for key, values in sorted(per_type.items())
        },
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="coverage_route_v1")
    parser.add_argument("--routed-question-types", default="completeness")
    parser.add_argument("--keep-baseline", type=int, default=9)
    parser.add_argument("--candidate-start", type=int, default=15)
    parser.add_argument("--candidate-scan", type=int, default=80)
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    if args.keep_baseline < 0:
        parser.error("--keep-baseline must be non-negative")
    if args.candidate_start < 0:
        parser.error("--candidate-start must be non-negative")
    if args.candidate_scan <= 0:
        parser.error("--candidate-scan must be positive")
    if args.limit <= 0:
        parser.error("--limit must be positive")
    if args.keep_baseline > args.limit:
        parser.error("--keep-baseline must be <= --limit")
    return args


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
