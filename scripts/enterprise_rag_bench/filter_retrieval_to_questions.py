#!/usr/bin/env python3
"""Filter and order retrieval rows to match an EnterpriseRAG question file."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    questions = read_jsonl(args.questions_file)
    retrieval_by_id = {
        str(row.get("question_id")): row for row in read_jsonl(args.retrieval_file) if row.get("question_id")
    }
    missing: list[str] = []
    rows: list[dict[str, Any]] = []
    for question in questions:
        qid = str(question.get("question_id"))
        row = retrieval_by_id.get(qid)
        if row is None:
            missing.append(qid)
            continue
        rows.append(row)
    if missing:
        raise SystemExit(f"missing retrieval rows for {len(missing)} questions: {missing[:10]}")
    write_jsonl(args.output, rows)
    print(
        json.dumps(
            {
                "output": str(args.output),
                "questions": len(questions),
                "retrieval_rows": len(rows),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
