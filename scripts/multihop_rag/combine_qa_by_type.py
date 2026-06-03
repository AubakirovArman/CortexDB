#!/usr/bin/env python3
"""Combine MultiHop-RAG QA outputs by replacing one question type."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from analyze_temporal_subtypes import temporal_answer_form, temporal_subtype


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def query_key(row: dict[str, Any]) -> str:
    return str(row.get("query", ""))


def row_matches(
    row: dict[str, Any],
    question_type: str,
    temporal_subtype_filter: str | None,
    temporal_answer_form_filter: str | None,
) -> bool:
    if row.get("question_type") != question_type:
        return False
    query = str(row.get("query", ""))
    if temporal_subtype_filter and temporal_subtype(query) != temporal_subtype_filter:
        return False
    if temporal_answer_form_filter and temporal_answer_form(query) != temporal_answer_form_filter:
        return False
    return True


def route_label(
    question_type: str,
    temporal_subtype_filter: str | None,
    temporal_answer_form_filter: str | None,
) -> str:
    parts = [question_type]
    if temporal_subtype_filter:
        parts.append(temporal_subtype_filter)
    if temporal_answer_form_filter:
        parts.append(temporal_answer_form_filter)
    parts.append("replacement")
    return ":".join(parts)


def combine(
    base_rows: list[dict[str, Any]],
    replacement_rows: list[dict[str, Any]],
    question_type: str,
    temporal_subtype_filter: str | None = None,
    temporal_answer_form_filter: str | None = None,
) -> list[dict[str, Any]]:
    replacements = {
        query_key(row): row
        for row in replacement_rows
        if row_matches(row, question_type, temporal_subtype_filter, temporal_answer_form_filter)
    }
    combined = []
    missing = []
    for row in base_rows:
        if row_matches(row, question_type, temporal_subtype_filter, temporal_answer_form_filter):
            replacement = replacements.get(query_key(row))
            if replacement is None:
                missing.append(query_key(row))
                combined.append(row)
            else:
                merged = dict(replacement)
                merged["prompt_route"] = route_label(
                    question_type,
                    temporal_subtype_filter,
                    temporal_answer_form_filter,
                )
                combined.append(merged)
        else:
            merged = dict(row)
            merged["prompt_route"] = "base"
            combined.append(merged)
    if missing:
        raise SystemExit(f"missing {len(missing)} replacement rows for {question_type}")
    return combined


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-qa", type=Path, required=True)
    parser.add_argument("--replacement-qa", type=Path, required=True)
    parser.add_argument("--question-type", required=True)
    parser.add_argument(
        "--temporal-subtype",
        choices=["change_over_time", "chronology", "consistency_conflict", "source_or_entity", "other"],
    )
    parser.add_argument(
        "--temporal-answer-form",
        choices=["yes_no", "choice", "temporal_label", "other"],
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    base_rows = read_json(args.base_qa)
    replacement_rows = read_json(args.replacement_qa)
    combined = combine(
        base_rows,
        replacement_rows,
        args.question_type,
        args.temporal_subtype,
        args.temporal_answer_form,
    )
    write_json(args.output, combined)
    replaced = sum(1 for row in combined if row.get("prompt_route") != "base")
    report = {
        "schema_version": "cortexdb.multihop_rag.qa_combination.v1",
        "base_qa": str(args.base_qa),
        "replacement_qa": str(args.replacement_qa),
        "question_type": args.question_type,
        "temporal_subtype": args.temporal_subtype,
        "temporal_answer_form": args.temporal_answer_form,
        "output": str(args.output),
        "rows": len(combined),
        "replaced_rows": replaced,
    }
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
