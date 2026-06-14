#!/usr/bin/env python3
"""Post-process EnterpriseRAG retrieval rows with deterministic local policies."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from oracle_free_abstain import abstain_decision


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


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval = rows_by_id(read_jsonl(args.retrieval_file), "retrieval")
    abstain_types = {item.strip() for item in args.abstain_question_types.split(",") if item.strip()}
    output_rows: list[dict[str, Any]] = []
    changed_rows = 0
    removed_docs = 0
    abstain_reasons: dict[str, int] = {}

    for qid, question in questions.items():
        row = dict(retrieval[qid])
        qtype = str(question.get("question_type") or "")
        qtext = str(question.get("question") or "")
        docs = [str(item) for item in row.get("document_ids", []) if str(item)]
        should_abstain = False
        reason = ""
        if args.oracle_free_abstain:
            should_abstain, reason = abstain_decision(
                question=qtext,
                document_ids=docs,
            )
        elif qtype in abstain_types:
            should_abstain = True
            reason = f"question_type:{qtype}"

        if should_abstain:
            removed_docs += len(docs)
            row["document_ids"] = []
            row["route"] = {
                "policy": args.policy_name,
                "source": "abstain",
                "reason": reason,
            }
            changed_rows += 1
            abstain_reasons[reason] = abstain_reasons.get(reason, 0) + 1
        output_rows.append(row)

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_postprocess.v1",
        "policy_name": args.policy_name,
        "retrieval_file": str(args.retrieval_file),
        "output": str(args.output),
        "abstain_question_types": sorted(abstain_types),
        "oracle_free_abstain": args.oracle_free_abstain,
        "questions": len(output_rows),
        "changed_rows": changed_rows,
        "removed_docs": removed_docs,
        "abstain_reasons": abstain_reasons,
        "note": "Local deterministic postprocess; no LLM/API calls.",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return report


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="v18_not_found_abstain")
    parser.add_argument("--abstain-question-types", default="info_not_found")
    parser.add_argument(
        "--oracle-free-abstain",
        action="store_true",
        help="Use the oracle-free abstain classifier instead of --abstain-question-types.",
    )
    return parser.parse_args(argv)


if __name__ == "__main__":
    run(parse_args())
