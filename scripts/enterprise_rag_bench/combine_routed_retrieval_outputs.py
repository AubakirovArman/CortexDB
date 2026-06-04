#!/usr/bin/env python3
"""Combine EnterpriseRAG retrieval artifacts with deterministic question routing."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


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


def require_same_ids(left: dict[str, Any], right: dict[str, Any], label: str) -> None:
    if set(left) != set(right):
        mismatch = sorted(set(left) ^ set(right))[:10]
        raise ValueError(f"{label} question_id mismatch: {mismatch}")


def route_source(question_type: str | None, routed_types: set[str]) -> str:
    return "routed" if str(question_type or "") in routed_types else "default"


def doc_ids(row: dict[str, Any]) -> list[str]:
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def recall_pct(question: dict[str, Any], row: dict[str, Any]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", [])}
    if not expected:
        return None
    retrieved = set(doc_ids(row))
    return round(len(expected & retrieved) / len(expected) * 100.0, 2)


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def run(args: argparse.Namespace) -> dict[str, Any]:
    routed_types = {item.strip() for item in args.routed_question_types.split(",") if item.strip()}
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    default_rows = rows_by_id(read_jsonl(args.default_retrieval_file), "default retrieval")
    routed_rows = rows_by_id(read_jsonl(args.routed_retrieval_file), "routed retrieval")
    require_same_ids(questions, default_rows, "default retrieval")
    require_same_ids(questions, routed_rows, "routed retrieval")

    output_rows: list[dict[str, Any]] = []
    route_counts = {"default": 0, "routed": 0}
    changed_rows = 0
    recall_values: list[float] = []

    for qid, question in questions.items():
        question_type = question.get("question_type")
        source = route_source(str(question_type), routed_types)
        route_counts[source] += 1
        selected = routed_rows[qid] if source == "routed" else default_rows[qid]
        baseline = default_rows[qid]
        row = dict(selected)
        row["route"] = {
            "policy": args.policy_name,
            "source": source,
            "question_type": question_type,
        }
        if doc_ids(row) != doc_ids(baseline):
            changed_rows += 1
        recall = recall_pct(question, row)
        if recall is not None:
            recall_values.append(recall)
        output_rows.append(row)

    write_jsonl(args.output, output_rows)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.routed_retrieval_report.v1",
        "policy_name": args.policy_name,
        "routed_question_types": sorted(routed_types),
        "questions_file": str(args.questions_file),
        "default_retrieval_file": str(args.default_retrieval_file),
        "routed_retrieval_file": str(args.routed_retrieval_file),
        "output": str(args.output),
        "questions": len(output_rows),
        "route_counts": route_counts,
        "changed_rows": changed_rows,
        "average_recall_pct": mean(recall_values),
        "note": "This routes retrieval rows before fresh answer generation; it does not call an LLM.",
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--default-retrieval-file", type=Path, required=True)
    parser.add_argument("--routed-retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="v8_selective_lexical_generation")
    parser.add_argument(
        "--routed-question-types",
        default="basic,completeness,conflicting_info,constrained,project_related",
    )
    return parser.parse_args()


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
