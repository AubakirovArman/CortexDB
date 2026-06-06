#!/usr/bin/env python3
"""Validate CortexDB LoCoMo retrieval-only adapter evidence."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


DATA_SCHEMA = "cortexdb.locomo.official_data_manifest.v1"
RETRIEVAL_SCHEMA = "cortexdb.locomo.retrieval_report.v1"


def fail(message: str) -> None:
    raise RuntimeError(message)


def load_json(path: Path) -> Any:
    if not path.exists():
        fail(f"missing file: {path}")
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        fail(f"missing file: {path}")
    rows = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict):
            fail(f"{path}:{line_number}: expected object")
        rows.append(row)
    return rows


def validate_manifest(manifest: dict[str, Any], min_samples: int) -> dict[str, Any]:
    if manifest.get("schema_version") != DATA_SCHEMA:
        fail("unexpected LoCoMo data manifest schema")
    files = manifest.get("files")
    if not isinstance(files, list) or not files:
        fail("data manifest missing files")
    item = files[0]
    samples = int(item.get("samples", 0))
    if samples < min_samples:
        fail(f"sample count {samples} below required {min_samples}")
    for field in ["qa_count", "qa_with_evidence", "turn_count", "sha256", "source_url"]:
        if not item.get(field):
            fail(f"data manifest missing {field}")
    return {
        "dataset": manifest.get("dataset"),
        "samples": samples,
        "qa_count": int(item["qa_count"]),
        "qa_with_evidence": int(item["qa_with_evidence"]),
        "turn_count": int(item["turn_count"]),
    }


def validate_runner_report(report: dict[str, Any], min_questions: int) -> dict[str, Any]:
    if report.get("schema_version") != RETRIEVAL_SCHEMA:
        fail("unexpected LoCoMo retrieval report schema")
    questions = int(report.get("questions", 0))
    if questions < min_questions:
        fail(f"retrieval questions {questions} below required {min_questions}")
    for field in ["samples", "turns_indexed", "qa_with_evidence", "top_k"]:
        if int(report.get(field, 0)) <= 0:
            fail(f"retrieval report missing positive {field}")
    return {
        "samples": int(report["samples"]),
        "questions": questions,
        "qa_with_evidence": int(report["qa_with_evidence"]),
        "turns_indexed": int(report["turns_indexed"]),
        "top_k": int(report["top_k"]),
        "output": report.get("output"),
        "db_root": report.get("db_root"),
    }


def evidence_hit(row: dict[str, Any], k: int) -> bool:
    evidence = set(str(item) for item in row.get("evidence_dialog_ids", []))
    retrieved = [str(item.get("dia_id")) for item in row.get("retrieval_list", [])[:k]]
    return bool(evidence.intersection(retrieved))


def compute_metrics(rows: list[dict[str, Any]], top_k: int) -> dict[str, Any]:
    if not rows:
        fail("retrieval output is empty")
    with_evidence = [row for row in rows if row.get("evidence_dialog_ids")]
    if not with_evidence:
        fail("retrieval output has no evidence-bearing rows")
    by_category: dict[str, list[bool]] = defaultdict(list)
    for row in with_evidence:
        if not row.get("question_id") or not row.get("sample_id"):
            fail("retrieval row missing question_id or sample_id")
        retrieved = row.get("retrieval_list")
        if not isinstance(retrieved, list):
            fail(f"retrieval row {row.get('question_id')} missing retrieval_list")
        by_category[str(row.get("category", "unknown"))].append(evidence_hit(row, top_k))
    hit_at_1 = sum(evidence_hit(row, 1) for row in with_evidence) / len(with_evidence)
    hit_at_k = sum(evidence_hit(row, top_k) for row in with_evidence) / len(with_evidence)
    return {
        "rows": len(rows),
        "rows_with_evidence": len(with_evidence),
        "hit@1": hit_at_1,
        f"hit@{top_k}": hit_at_k,
        "by_category": {
            category: {
                "count": len(values),
                f"hit@{top_k}": sum(values) / len(values),
            }
            for category, values in sorted(by_category.items())
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-manifest", type=Path, required=True)
    parser.add_argument("--retrieval-report", type=Path, required=True)
    parser.add_argument("--retrieval-output", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--min-samples", type=int, default=10)
    parser.add_argument("--min-questions", type=int, default=1000)
    args = parser.parse_args()

    manifest_summary = validate_manifest(load_json(args.data_manifest), args.min_samples)
    runner_summary = validate_runner_report(load_json(args.retrieval_report), args.min_questions)
    metrics = compute_metrics(read_jsonl(args.retrieval_output), runner_summary["top_k"])
    if metrics["rows"] != runner_summary["questions"]:
        fail("retrieval output row count does not match runner report")
    report = {
        "schema_version": "cortexdb.locomo.retrieval_adapter_check.v1",
        "status": "passed",
        "official": {
            "dataset": manifest_summary,
            "source": "snap-research/locomo data/locomo10.json",
        },
        "cortexdb": {**runner_summary, "metrics": metrics},
        "claims_boundary": [
            "retrieval-only LoCoMo evidence",
            "not an end-to-end QA claim",
            "not a published LoCoMo leaderboard entry",
            "no API calls are made by this checker",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
