#!/usr/bin/env python3
"""Validate CortexDB LongMemEval v1 retrieval-adapter evidence."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


EXPECTED_DATASET = "xiaowu0162/longmemeval-cleaned"
EXPECTED_REPORT_SCHEMA = "cortexdb.longmemeval.v1.retrieval_report.v1"
EXPECTED_MANIFEST_SCHEMA = "cortexdb.longmemeval.v1.official_data_manifest.v1"


def fail(message: str) -> None:
    raise RuntimeError(message)


def load_json(path: Path) -> Any:
    if not path.exists():
        fail(f"missing file: {path}")
    return json.loads(path.read_text(encoding="utf-8"))


def count_jsonl(path: Path) -> int:
    if not path.exists():
        fail(f"missing file: {path}")
    count = 0
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                fail(f"{path}:{line_number}: expected JSON object")
            if not row.get("question_id"):
                fail(f"{path}:{line_number}: missing question_id")
            results = row.get("retrieval_results")
            if not isinstance(results, dict):
                fail(f"{path}:{line_number}: missing retrieval_results object")
            ranked = results.get("ranked_items")
            if not isinstance(ranked, list):
                fail(f"{path}:{line_number}: missing ranked_items list")
            count += 1
    return count


def parse_official_metrics(path: Path) -> dict[str, float]:
    if not path.exists():
        fail(f"missing file: {path}")
    text = path.read_text(encoding="utf-8")
    metrics: dict[str, float] = {}
    for name, value in re.findall(r"([a-z_]+@\d+)\s*=\s*([0-9.]+)", text):
        metrics[f"session {name}"] = float(value)
    required = [
        "session recall_all@5",
        "session ndcg_any@5",
        "session recall_all@10",
        "session ndcg_any@10",
    ]
    for name in required:
        if name not in metrics:
            fail(f"{path}: missing official metric {name}")
    return metrics


def validate_manifest(manifest: dict[str, Any], min_rows: int) -> dict[str, Any]:
    if manifest.get("schema_version") != EXPECTED_MANIFEST_SCHEMA:
        fail("unexpected LongMemEval data manifest schema")
    if manifest.get("dataset") != EXPECTED_DATASET:
        fail("unexpected LongMemEval dataset name")
    if manifest.get("split") != "s":
        fail("Epic 49 requires the LongMemEval-S split")
    files = manifest.get("files")
    if not isinstance(files, list) or not files:
        fail("data manifest has no files")
    rows = int(files[0].get("rows", 0))
    if rows < min_rows:
        fail(f"data manifest rows {rows} below required {min_rows}")
    if not files[0].get("sha256"):
        fail("data manifest missing sha256")
    return {"dataset": manifest["dataset"], "split": manifest["split"], "rows": rows}


def validate_report(report: dict[str, Any], min_rows: int) -> dict[str, Any]:
    if report.get("schema_version") != EXPECTED_REPORT_SCHEMA:
        fail("unexpected LongMemEval retrieval report schema")
    if report.get("status") != "passed":
        fail("retrieval report did not pass")
    summary = report.get("summary")
    if not isinstance(summary, dict):
        fail("retrieval report missing summary")
    question_count = int(summary.get("question_count", 0))
    aggregate_count = int(summary.get("aggregate_count", 0))
    if question_count < min_rows:
        fail(f"retrieval question_count {question_count} below required {min_rows}")
    if aggregate_count <= 0:
        fail("retrieval aggregate_count must be positive")
    metrics = summary.get("metrics")
    if not isinstance(metrics, dict):
        fail("retrieval report missing metrics")
    for name in ["recall_all@10", "ndcg_any@10"]:
        value = metrics.get(name)
        if not isinstance(value, (float, int)):
            fail(f"retrieval report missing numeric metric {name}")
    return {
        "question_count": question_count,
        "aggregate_count": aggregate_count,
        "top_k": int(report.get("top_k", 0)),
        "granularity": summary.get("granularity", ""),
        "internal_metrics": {
            "recall_all@10": float(metrics["recall_all@10"]),
            "ndcg_any@10": float(metrics["ndcg_any@10"]),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-manifest", type=Path, required=True)
    parser.add_argument("--retrieval-report", type=Path, required=True)
    parser.add_argument("--retrieval-log", type=Path, required=True)
    parser.add_argument("--official-metrics", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--min-rows", type=int, default=500)
    args = parser.parse_args()

    if args.min_rows <= 0:
        fail("--min-rows must be positive")
    manifest_summary = validate_manifest(load_json(args.data_manifest), args.min_rows)
    retrieval_summary = validate_report(load_json(args.retrieval_report), args.min_rows)
    row_count = count_jsonl(args.retrieval_log)
    if row_count != retrieval_summary["question_count"]:
        fail(
            "retrieval log row count does not match report question_count: "
            f"{row_count} != {retrieval_summary['question_count']}"
        )
    official_metrics = parse_official_metrics(args.official_metrics)

    report = {
        "schema_version": "cortexdb.longmemeval.v1.retrieval_adapter_check.v1",
        "status": "passed",
        "official": {
            "dataset": manifest_summary,
            "metrics": official_metrics,
            "retrieval_metric_script": "LongMemEval/src/evaluation/print_retrieval_metrics.py",
        },
        "cortexdb": {
            "retrieval_report": str(args.retrieval_report),
            "retrieval_log": str(args.retrieval_log),
            "retrieval_log_rows": row_count,
            **retrieval_summary,
        },
        "claims_boundary": [
            "retrieval-only LongMemEval v1 evidence",
            "not an official leaderboard entry until submitted",
            "not an end-to-end QA claim",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
