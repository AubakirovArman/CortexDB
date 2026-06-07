#!/usr/bin/env python3
"""Validate the LongMemEval evidence page for Epic 137."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


TASK_MARKERS = {
    "retrieval_only_results": [
        "## Retrieval-only Results",
        "recall_all@10 = 0.9021",
        "ndcg_any@10 = 0.7873",
        "These are retrieval metrics. They are not the final QA leaderboard score.",
    ],
    "official_evaluator_command": [
        "## Official Evaluator Command",
        "make longmemeval-v1-official-retrieval-metrics",
        "LongMemEval/src/evaluation/print_retrieval_metrics.py",
        "make longmemeval-v1-retrieval-adapter-check",
    ],
    "log_format": [
        "## Retrieval Log Format",
        "longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl",
        '"question_id": "string"',
        '"retrieval_results"',
        '"ranked_items"',
    ],
    "limitations": [
        "## Limitations",
        "not a public LongMemEval leaderboard/list placement",
        "DeepSeek Flash runs are local diagnostics",
        "not committed to the repository",
        "submitted to the LongMemEval maintainers",
    ],
}

REQUIRED_DOC_MARKERS = [
    "## Epic 137 Evidence Page Contract",
    "target/longmemeval-v1/cortexdb/report.json",
    "target/longmemeval-v1/retrieval-adapter/report.json",
    "target/longmemeval-v1/e2e-adapter/report.json",
    "schema: cortexdb.longmemeval.v1.retrieval_adapter_check.v1",
    "retrieval-only, not an end-to-end QA claim",
    "not an official published LongMemEval leaderboard entry",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page", type=Path, default=Path("docs/LONGMEMEVAL_OFFICIAL.md"))
    parser.add_argument("--report", type=Path, default=Path("target/longmemeval-v1/evidence-page/report.json"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    if not args.page.is_file():
        failures.append(f"missing page {args.page}")
        text = ""
    else:
        text = args.page.read_text(encoding="utf-8")

    for marker in REQUIRED_DOC_MARKERS:
        if marker not in text:
            failures.append(f"{args.page}: missing {marker!r}")

    task_coverage: dict[str, bool] = {}
    for task, markers in TASK_MARKERS.items():
        task_coverage[task] = all(marker in text for marker in markers)
        if not task_coverage[task]:
            failures.append(f"{args.page}: Epic 137 task not covered: {task}")

    report = {
        "schema_version": "cortexdb.longmemeval.evidence_page.report.v1",
        "status": "failed" if failures else "passed",
        "page": str(args.page),
        "task_coverage": task_coverage,
        "failures": failures,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"LongMemEval evidence page check failed: {args.report}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"LongMemEval evidence page check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
