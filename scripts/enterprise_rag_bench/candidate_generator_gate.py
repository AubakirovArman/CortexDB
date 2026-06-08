#!/usr/bin/env python3
"""Validate EnterpriseRAG candidate-generator depth thresholds."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("report must be a JSON object")
    return value


def metric(report: dict[str, Any], depth: int, key: str) -> float:
    stats = report.get("depth_stats", {}).get(str(depth), {})
    value = stats.get(key)
    if value is None:
        raise ValueError(f"missing depth_stats.{depth}.{key}")
    return float(value)


def run(args: argparse.Namespace) -> dict[str, Any]:
    report = read_json(args.depth_report)
    recall_500 = metric(report, 500, "average_recall_pct")
    recall_1000 = metric(report, 1000, "average_recall_pct")
    full_1000 = metric(report, 1000, "full_recall_questions")
    hit_1000 = metric(report, 1000, "hit_questions")
    passed = (
        recall_500 >= args.min_recall_500
        and recall_1000 >= args.min_recall_1000
        and full_1000 >= args.min_full_recall_1000
    )
    result = {
        "schema_version": "cortexdb.enterprise_rag_bench.candidate_generator_gate.v1",
        "depth_report": str(args.depth_report),
        "passed": passed,
        "thresholds": {
            "min_recall_500": args.min_recall_500,
            "min_recall_1000": args.min_recall_1000,
            "min_full_recall_1000": args.min_full_recall_1000,
        },
        "metrics": {
            "recall_500": recall_500,
            "recall_1000": recall_1000,
            "full_recall_1000": int(full_1000),
            "hit_questions_1000": int(hit_1000),
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not passed:
        raise SystemExit(2)
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--depth-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--min-recall-500", type=float, default=85.0)
    parser.add_argument("--min-recall-1000", type=float, default=90.0)
    parser.add_argument("--min-full-recall-1000", type=float, default=400.0)
    return parser.parse_args()


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
