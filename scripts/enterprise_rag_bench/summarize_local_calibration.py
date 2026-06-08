#!/usr/bin/env python3
"""Summarize local EnterpriseRAG retrieval calibration artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# EnterpriseRAG Local Calibration Gate",
        "",
        "This report is local-only: no LLM calls and no external API calls.",
        "",
        "## Result",
        "",
        f"- passed: `{str(report['passed']).lower()}`",
        f"- retrieval artifact: `{report['retrieval_file']}`",
        f"- top10 document recall: `{report['top10_document_recall_pct']}`",
        f"- top10 full-recall questions: `{report['top10_full_recall_questions']}`",
        f"- top10 hit questions: `{report['top10_hit_questions']}`",
        f"- average invalid extra docs: `{report['average_invalid_extra_docs']}`",
        f"- fact token coverage proxy: `{report['average_fact_token_coverage_pct']}`",
        "",
        "## Thresholds",
        "",
        f"- min top10 recall: `{report['thresholds']['min_top10_recall_pct']}`",
        f"- max invalid extra docs: `{report['thresholds']['max_invalid_extra_docs']}`",
        "",
        "## Notes",
        "",
        "- This is a retrieval/evidence calibration gate, not an official answer-generation score.",
        "- Answer quality still requires a separate LLM run and official judge/evaluator.",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run(args: argparse.Namespace) -> dict[str, Any]:
    depth = read_json(args.depth_report)
    evidence = read_json(args.evidence_report)
    top10 = depth["depth_stats"]["10"]
    top10_recall = float(top10["average_recall_pct"])
    invalid_extra = float(evidence["average_invalid_extra_docs"])
    passed = top10_recall >= args.min_top10_recall_pct and invalid_extra <= args.max_invalid_extra_docs
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.local_calibration_gate.v1",
        "passed": passed,
        "retrieval_file": evidence["retrieval_file"],
        "depth_report": str(args.depth_report),
        "evidence_report": str(args.evidence_report),
        "top10_document_recall_pct": top10_recall,
        "top10_full_recall_questions": int(top10["full_recall_questions"]),
        "top10_hit_questions": int(top10["hit_questions"]),
        "average_invalid_extra_docs": invalid_extra,
        "average_fact_token_coverage_pct": float(evidence["average_fact_token_coverage_pct"]),
        "average_fact_full_coverage_pct": float(evidence["average_fact_full_coverage_pct"]),
        "thresholds": {
            "min_top10_recall_pct": args.min_top10_recall_pct,
            "max_invalid_extra_docs": args.max_invalid_extra_docs,
        },
        "note": "Local retrieval/evidence gate only; no LLM/API calls.",
    }
    write_json(args.output, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    print(json.dumps(report, sort_keys=True))
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--depth-report", type=Path, required=True)
    parser.add_argument("--evidence-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--min-top10-recall-pct", type=float, default=69.8)
    parser.add_argument("--max-invalid-extra-docs", type=float, default=8.5)
    return parser.parse_args()


if __name__ == "__main__":
    run(parse_args())
