#!/usr/bin/env python3
"""Apply type-specific top-k caps to EnterpriseRAG retrieval rows."""

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


def parse_cap(value: str) -> tuple[str, int]:
    if "=" not in value:
        raise argparse.ArgumentTypeError("cap must be QUESTION_TYPE=K")
    question_type, raw_limit = value.split("=", 1)
    question_type = question_type.strip()
    if not question_type:
        raise argparse.ArgumentTypeError("question type cannot be empty")
    try:
        limit = int(raw_limit)
    except ValueError as error:
        raise argparse.ArgumentTypeError("cap K must be an integer") from error
    if limit < 0:
        raise argparse.ArgumentTypeError("cap K must be non-negative")
    return question_type, limit


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    caps = dict(args.cap)
    output_rows: list[dict[str, Any]] = []
    changed_rows = 0
    recall_values: list[float] = []
    invalid_values: list[int] = []
    per_type: dict[str, list[float]] = {}

    for row in read_jsonl(args.retrieval_file):
        qid = str(row.get("question_id"))
        question = questions.get(qid, row)
        qtype = str(question.get("question_type") or "unknown")
        limit = caps.get(qtype, args.default_limit)
        docs = [str(item) for item in row.get("document_ids", []) if str(item)][:limit]
        expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
        invalid_values.append(len([doc_id for doc_id in docs if doc_id not in expected]))
        recall = recall_pct(question, docs)
        if recall is not None:
            recall_values.append(recall)
            per_type.setdefault(qtype, []).append(recall)
        output = dict(row)
        if docs != row.get("document_ids", []):
            changed_rows += 1
        output["document_ids"] = docs
        output["topk_policy"] = {
            "policy": args.policy_name,
            "question_type": qtype,
            "limit": limit,
        }
        output_rows.append(output)

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.type_topk_policy.v1",
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "output": str(args.output),
        "policy_name": args.policy_name,
        "default_limit": args.default_limit,
        "caps": caps,
        "changed_rows": changed_rows,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "average_invalid_extra_docs_proxy": round(sum(invalid_values) / len(invalid_values), 2)
        if invalid_values
        else 0.0,
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
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="type_topk_policy_v1")
    parser.add_argument("--default-limit", type=int, default=10)
    parser.add_argument("--cap", type=parse_cap, action="append", default=[])
    args = parser.parse_args()
    if args.default_limit < 0:
        parser.error("--default-limit must be non-negative")
    return args


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
