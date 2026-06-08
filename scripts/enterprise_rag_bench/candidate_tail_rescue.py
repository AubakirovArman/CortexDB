#!/usr/bin/env python3
"""Inject early candidate-pool documents into final top-k tails.

This is a deterministic local EnterpriseRAG-Bench retrieval postprocess. It is
intended for cases where the candidate generator already found plausible
documents, but the final top10 kept a near-duplicate or stale variant instead.
It does not call an LLM/API and it does not use gold labels to select docs.
"""

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


def split_csv(value: str) -> set[str]:
    return {item.strip() for item in value.split(",") if item.strip()}


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def route_enabled(
    *,
    question: dict[str, Any],
    route_question_types: set[str],
    route_source_types: set[str],
) -> bool:
    qtype = str(question.get("question_type") or "")
    if route_question_types and qtype not in route_question_types:
        return False
    if not route_source_types:
        return True
    question_sources = {str(item) for item in question.get("source_types", []) if str(item)}
    return bool(question_sources & route_source_types)


def rescue_tail(
    *,
    baseline_ids: list[str],
    candidate_ids: list[str],
    limit: int,
    tail_slots: int,
    candidate_rank_limit: int,
) -> tuple[list[str], list[str]]:
    if tail_slots <= 0:
        return baseline_ids[:limit], []
    protected = baseline_ids[: max(0, limit - tail_slots)]
    seen = set(protected)
    injected: list[str] = []
    for doc_id in candidate_ids[:candidate_rank_limit]:
        if doc_id in seen:
            continue
        injected.append(doc_id)
        seen.add(doc_id)
        if len(injected) >= tail_slots:
            break
    tail: list[str] = []
    for doc_id in baseline_ids:
        if len(protected) + len(injected) + len(tail) >= limit:
            break
        if doc_id in seen:
            continue
        tail.append(doc_id)
        seen.add(doc_id)
    return (protected + injected + tail)[:limit], injected


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    candidates = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidates")
    route_question_types = split_csv(args.route_question_types)
    route_source_types = split_csv(args.route_source_types)

    output_rows: list[dict[str, Any]] = []
    changed_rows = 0
    routed_rows = 0
    injected_docs = 0
    recall_values: list[float] = []
    per_type: dict[str, list[float]] = defaultdict(list)

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        candidate_ids = doc_ids(candidates.get(qid))
        output = dict(base_row)
        if route_enabled(
            question=question,
            route_question_types=route_question_types,
            route_source_types=route_source_types,
        ):
            routed_rows += 1
            selected, injected = rescue_tail(
                baseline_ids=baseline_ids,
                candidate_ids=candidate_ids,
                limit=args.limit,
                tail_slots=args.tail_slots,
                candidate_rank_limit=args.candidate_rank_limit,
            )
            if selected != baseline_ids:
                changed_rows += 1
            injected_docs += len(injected)
            output["document_ids"] = selected
            output["tail_rescue"] = {
                "policy": args.policy_name,
                "enabled": True,
                "injected_doc_ids": injected,
                "question_type": question.get("question_type"),
                "source_types": question.get("source_types", []),
            }
        else:
            output["document_ids"] = baseline_ids
            output["tail_rescue"] = {
                "policy": args.policy_name,
                "enabled": False,
                "question_type": question.get("question_type"),
                "source_types": question.get("source_types", []),
            }
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)
            per_type[str(question.get("question_type") or "unknown")].append(recall)

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.candidate_tail_rescue.v1",
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "output": str(args.output),
        "route_question_types": sorted(route_question_types),
        "route_source_types": sorted(route_source_types),
        "tail_slots": args.tail_slots,
        "candidate_rank_limit": args.candidate_rank_limit,
        "changed_rows": changed_rows,
        "routed_rows": routed_rows,
        "injected_docs": injected_docs,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "per_type_recall_pct": {
            key: round(sum(values) / len(values), 2)
            for key, values in sorted(per_type.items())
        },
        "note": "Local deterministic tail rescue; no LLM/API calls and no gold-aware selection.",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="candidate_tail_rescue_v1")
    parser.add_argument("--route-question-types", default="")
    parser.add_argument("--route-source-types", default="")
    parser.add_argument("--tail-slots", type=int, default=1)
    parser.add_argument("--candidate-rank-limit", type=int, default=50)
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    if args.tail_slots < 0:
        parser.error("--tail-slots must be non-negative")
    if args.candidate_rank_limit < 1:
        parser.error("--candidate-rank-limit must be positive")
    if args.limit < 1:
        parser.error("--limit must be positive")
    if args.tail_slots > args.limit:
        parser.error("--tail-slots cannot exceed --limit")
    return args


def main() -> int:
    run(parse_args())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
