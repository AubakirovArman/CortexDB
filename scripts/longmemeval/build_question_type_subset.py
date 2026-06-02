#!/usr/bin/env python3
"""Build a LongMemEval retrieval/reference subset by question type."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def run(args: argparse.Namespace) -> dict[str, Any]:
    refs = [row for row in read_json(args.reference_file) if row.get("question_type") == args.question_type]
    wanted = {row["question_id"] for row in refs}
    retrieval_rows = [row for row in read_jsonl(args.retrieval_log) if row.get("question_id") in wanted]
    found = {row["question_id"] for row in retrieval_rows}
    missing = sorted(wanted - found)
    if missing:
        raise RuntimeError(f"missing retrieval rows for {len(missing)} question IDs: {missing[:10]}")
    args.output_root.mkdir(parents=True, exist_ok=True)
    reference_out = args.output_root / f"{args.output_prefix}_reference.json"
    retrieval_out = args.output_root / f"{args.output_prefix}_retrieval.jsonl"
    write_json(reference_out, refs)
    write_jsonl(retrieval_out, retrieval_rows)
    report = {
        "schema_version": "cortexdb.longmemeval.v1.question_type_subset.v1",
        "question_type": args.question_type,
        "questions": len(refs),
        "retrieval_rows": len(retrieval_rows),
        "reference_file": str(reference_out),
        "retrieval_log": str(retrieval_out),
    }
    write_json(args.output_root / f"{args.output_prefix}_subset_report.json", report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-file", type=Path, required=True)
    parser.add_argument("--retrieval-log", type=Path, required=True)
    parser.add_argument("--question-type", required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--output-prefix", default="subset")
    return parser.parse_args()


def main() -> int:
    print(json.dumps(run(parse_args()), ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
