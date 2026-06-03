#!/usr/bin/env python3
"""Post-process MultiHop-RAG QA answers without issuing model calls."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from qa_prompting import normalize_temporal_answer_for_question


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def normalize_temporal_rows(rows: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], int]:
    normalized = []
    changed = 0
    for row in rows:
        next_row = dict(row)
        if row.get("question_type") == "temporal_query":
            answer = str(row.get("model_answer", ""))
            normalized_answer = normalize_temporal_answer_for_question(str(row.get("query", "")), answer)
            if normalized_answer != answer:
                next_row["raw_model_answer"] = answer
                next_row["model_answer"] = normalized_answer
                next_row["temporal_answer_normalized"] = True
                changed += 1
        normalized.append(next_row)
    return normalized, changed


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--temporal-answer-normalize", action="store_true")
    args = parser.parse_args()
    rows = read_json(args.input)
    changed = 0
    if args.temporal_answer_normalize:
        rows, changed = normalize_temporal_rows(rows)
    write_json(args.output, rows)
    report = {
        "schema_version": "cortexdb.multihop_rag.qa_postprocess_report.v1",
        "input": str(args.input),
        "output": str(args.output),
        "rows": len(rows),
        "temporal_answer_normalize": args.temporal_answer_normalize,
        "temporal_answers_changed": changed,
    }
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
