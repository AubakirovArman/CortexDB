#!/usr/bin/env python3
"""Validate local EnterpriseRAG-Bench inputs for CortexDB runs."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number}: expected JSON object")
            rows.append(value)
    return rows


def source_type(path: str) -> str:
    return path.split("/", 1)[0] if "/" in path else "unknown"


def validate_questions(
    questions: list[dict[str, Any]],
    uuid_index: dict[str, str],
) -> tuple[dict[str, Any], list[str]]:
    failures: list[str] = []
    by_type: Counter[str] = Counter()
    expected_doc_counts: Counter[int] = Counter()
    seen_ids: set[str] = set()

    for index, row in enumerate(questions, 1):
        label = f"question row {index}"
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid.strip():
            failures.append(f"{label}: missing question_id")
            qid = f"row-{index}"
        elif qid in seen_ids:
            failures.append(f"{label}: duplicate question_id {qid}")
        seen_ids.add(str(qid))

        question = row.get("question")
        if not isinstance(question, str) or not question.strip():
            failures.append(f"{label}: missing question")

        question_type = row.get("question_type")
        if not isinstance(question_type, str) or not question_type.strip():
            failures.append(f"{label}: missing question_type")
            question_type = "unknown"
        by_type[str(question_type)] += 1

        expected = row.get("expected_doc_ids")
        if expected is None:
            expected = []
        if not isinstance(expected, list):
            failures.append(f"{label}: expected_doc_ids must be a list")
            expected = []
        expected_doc_counts[len(expected)] += 1
        for doc_id in expected:
            if not isinstance(doc_id, str):
                failures.append(f"{label}: expected_doc_ids contains non-string")
            elif doc_id not in uuid_index:
                failures.append(f"{label}: unknown expected_doc_id {doc_id}")

        answer_facts = row.get("answer_facts")
        if answer_facts is not None and not isinstance(answer_facts, list):
            failures.append(f"{label}: answer_facts must be a list when present")

    return (
        {
            "rows": len(questions),
            "by_question_type": dict(sorted(by_type.items())),
            "expected_doc_counts": {
                str(key): value for key, value in sorted(expected_doc_counts.items())
            },
        },
        failures,
    )


def validate_index(uuid_index: dict[str, str], sources_dir: Path) -> tuple[dict[str, Any], list[str]]:
    failures: list[str] = []
    by_source: Counter[str] = Counter()
    missing_samples = 0

    for doc_id, rel_path in uuid_index.items():
        if not isinstance(doc_id, str) or not doc_id.startswith("dsid_"):
            failures.append(f"invalid doc id in uuid index: {doc_id!r}")
            continue
        if not isinstance(rel_path, str) or not rel_path.endswith(".json"):
            failures.append(f"{doc_id}: invalid relative path {rel_path!r}")
            continue
        by_source[source_type(rel_path)] += 1

    for doc_id, rel_path in list(uuid_index.items())[:20]:
        if not (sources_dir / rel_path).is_file():
            missing_samples += 1
            failures.append(f"{doc_id}: source document not found at {rel_path}")

    return (
        {
            "documents": len(uuid_index),
            "source_types": dict(sorted(by_source.items())),
            "sample_missing_files": missing_samples,
        },
        failures,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bench-root", type=Path, required=True)
    parser.add_argument("--questions-file", type=Path)
    parser.add_argument("--uuid-index", type=Path)
    parser.add_argument("--sources-dir", type=Path)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()

    questions_path = args.questions_file or args.bench_root / "questions.jsonl"
    uuid_path = args.uuid_index or args.bench_root / "generated_data/uuid_index.json"
    sources_dir = args.sources_dir or args.bench_root / "generated_data/sources"

    questions = read_jsonl(questions_path)
    uuid_index = read_json(uuid_path)
    if not isinstance(uuid_index, dict):
        raise ValueError(f"{uuid_path}: expected JSON object")

    question_report, question_failures = validate_questions(questions, uuid_index)
    index_report, index_failures = validate_index(uuid_index, sources_dir)
    failures = question_failures + index_failures
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.preflight_report.v1",
        "status": "failed" if failures else "passed",
        "bench_root": str(args.bench_root),
        "questions_file": str(questions_path),
        "uuid_index": str(uuid_path),
        "sources_dir": str(sources_dir),
        "questions": question_report,
        "uuid_index_report": index_report,
        "failures": failures,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    if failures:
        print(f"EnterpriseRAG-Bench preflight failed: {args.report}")
        for failure in failures[:25]:
            print(f"- {failure}")
        return 1
    print(f"EnterpriseRAG-Bench preflight passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
