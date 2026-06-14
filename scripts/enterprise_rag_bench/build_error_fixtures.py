#!/usr/bin/env python3
"""Build labeled EnterpriseRAG-Bench error fixtures from official artifacts.

These fixtures are for local testing and regression gates; they may contain
oracle labels (question_type, expected_doc_ids, gold_answer) and must NOT be
used as inference inputs in an official-clean run.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def is_abstain(answer_text: str) -> bool:
    return "insufficient information" in answer_text.lower()


def build_fixtures(args: argparse.Namespace) -> dict[str, list[dict[str, Any]]]:
    questions = {str(row.get("question_id")): row for row in read_jsonl(args.questions_file)}
    answers = {str(row.get("question_id")): row for row in read_jsonl(args.answers_file)}
    metrics = read_json(args.metrics_file)
    metric_rows = {str(row.get("question_id")): row for row in metrics.get("questions", [])}

    fixtures: dict[str, list[dict[str, Any]]] = {
        "false_abstain": [],
        "high_level_abstain": [],
        "info_not_found_abstain": [],
        "project_related": [],
        "intra_document": [],
        "completeness": [],
        "constrained": [],
    }

    for qid in sorted(questions):
        question = questions[qid]
        answer = answers.get(qid, {})
        metric = metric_rows.get(qid, {})
        qtype = str(question.get("question_type") or "unknown")
        answer_text = str(answer.get("answer", ""))
        retrieved_count = len(answer.get("document_ids", []))

        base = {
            "question_id": qid,
            "category": qtype,
            "question": question.get("question", ""),
            "expected_behavior": "answered",
            "retrieved_doc_count": retrieved_count,
            "document_recall_pct": metric.get("document_recall_pct"),
            "answer_correct": metric.get("answer_correct"),
            "completeness_pct": metric.get("completeness_pct"),
            "candidate_answer_preview": answer_text[:500],
        }

        if qtype in {"basic", "semantic"} and is_abstain(answer_text) and retrieved_count > 0:
            row = dict(base)
            row["expected_behavior"] = "answered"
            row["notes"] = (
                "False abstain: retrieval returned documents but the answer still abstained. "
                "Guard/prompt should convert this to a substantive answer without harming info_not_found."
            )
            fixtures["false_abstain"].append(row)

        if qtype == "high_level" and is_abstain(answer_text):
            row = dict(base)
            row["expected_behavior"] = "answered"
            row["notes"] = (
                "High-level company question: do not apply zero-doc abstain. "
                "Route to company-scope documents and synthesize an answer from evidence."
            )
            fixtures["high_level_abstain"].append(row)

        if qtype == "info_not_found" and is_abstain(answer_text):
            row = dict(base)
            row["expected_behavior"] = "abstain"
            row["notes"] = (
                "Genuine info_not_found: the system must keep abstaining on these questions. "
                "Any abstain-chain change must preserve this behavior."
            )
            fixtures["info_not_found_abstain"].append(row)

        if qtype == "project_related":
            row = dict(base)
            row["notes"] = (
                "Project-related: answer must use exact mechanism/identifier names from evidence; "
                "evidence-first prompt and slot planner are expected to help here."
            )
            fixtures["project_related"].append(row)

        if qtype == "intra_document_reasoning":
            row = dict(base)
            row["notes"] = (
                "Intra-document reasoning: full-document context is expected to preserve links "
                "inside a single document."
            )
            fixtures["intra_document"].append(row)

        if qtype == "completeness":
            row = dict(base)
            row["notes"] = (
                "Completeness: every requested subpart/slot must be covered; slot planner should help."
            )
            fixtures["completeness"].append(row)

        if qtype == "constrained":
            row = dict(base)
            row["notes"] = (
                "Constrained: explicit scope filters must be respected and restated in the answer."
            )
            fixtures["constrained"].append(row)

    return fixtures


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--answers-file", type=Path, required=True)
    parser.add_argument("--metrics-file", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    fixtures = build_fixtures(args)
    summary: dict[str, int] = {}
    for name, rows in fixtures.items():
        output_path = args.output_dir / f"{name}.jsonl"
        write_jsonl(output_path, rows)
        summary[name] = len(rows)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    summary_path = args.output_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
