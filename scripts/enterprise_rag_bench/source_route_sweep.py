#!/usr/bin/env python3
"""Sweep source-type routed doc-view rerank variants.

This is a local-only EnterpriseRAG-Bench regression harness. It runs the
existing doc-view reranker for one source type at a time, compares each output
against a baseline retrieval artifact, and writes a compact promotion report.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Source Route Sweep",
        "",
        "This is a local retrieval-only sweep. It does not call an LLM or API.",
        "",
        f"- baseline: `{report['baseline_retrieval_file']}`",
        f"- candidate pool: `{report['candidate_retrieval_file']}`",
        f"- route question types: `{', '.join(report['route_question_types'])}`",
        "",
        "| Source Type | Decision | Recall Delta | Full Delta | Hit Delta | Improved | Regressed |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for item in report["results"]:
        metrics = item["metrics"]
        lines.append(
            "| `{source}` | `{decision}` | `{recall:+.2f}` | `{full:+d}` | `{hit:+d}` | `{improved}` | `{regressed}` |".format(
                source=item["source_type"],
                decision=item["decision"],
                recall=metrics["average_recall_pct"]["delta"],
                full=metrics["full_recall_questions"]["delta"],
                hit=metrics["hit_questions"]["delta"],
                improved=len(item["improved_question_ids"]),
                regressed=len(item["regressed_question_ids"]),
            )
        )
    lines.extend(
        [
            "",
            "Promotion rule:",
            "",
            "- `promote_candidate`: no regressed questions and a positive recall/full/hit delta.",
            "- `neutral`: no regressed questions and no positive metric delta.",
            "- `reject_regression`: at least one regressed question.",
            "",
        ]
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def run_command(args: list[str]) -> None:
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL)


def sanitize(value: str) -> str:
    return "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in value)


def decision_for(report: dict[str, Any]) -> str:
    if report.get("regressed_question_ids"):
        return "reject_regression"
    metrics = report["metrics"]
    if (
        metrics["average_recall_pct"]["delta"] > 0
        or metrics["full_recall_questions"]["delta"] > 0
        or metrics["hit_questions"]["delta"] > 0
    ):
        return "promote_candidate"
    return "neutral"


def run(args: argparse.Namespace) -> dict[str, Any]:
    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)
    results: list[dict[str, Any]] = []

    for source_type in args.source_types:
        safe_source = sanitize(source_type)
        retrieval_output = output_dir / f"{args.run_id}_{safe_source}.jsonl"
        rerank_report = output_dir / f"{args.run_id}_{safe_source}_rerank_report.json"
        compare_details = output_dir / f"{args.run_id}_{safe_source}_comparison.jsonl"
        compare_report = output_dir / f"{args.run_id}_{safe_source}_comparison.json"
        compare_markdown = output_dir / f"{args.run_id}_{safe_source}_comparison.md"

        run_command(
            [
                sys.executable,
                "scripts/enterprise_rag_bench/doc_view_rerank.py",
                "--questions-file",
                str(args.questions_file),
                "--retrieval-file",
                str(args.candidate_retrieval_file),
                "--baseline-retrieval-file",
                str(args.baseline_retrieval_file),
                "--uuid-index",
                str(args.uuid_index),
                "--sources-dir",
                str(args.sources_dir),
                "--doc-views-file",
                str(args.doc_views_file),
                "--embedding-cache",
                str(args.embedding_cache),
                "--output",
                str(retrieval_output),
                "--report",
                str(rerank_report),
                "--score-candidate-limit",
                str(args.score_candidate_limit),
                "--limit",
                str(args.limit),
                "--seed-count",
                str(args.seed_count),
                "--protect-baseline-prefix",
                str(args.protect_baseline_prefix),
                "--route-question-types",
                ",".join(args.route_question_types),
                "--route-source-types",
                source_type,
                "--diagnostics-top-k",
                "0",
            ]
        )
        run_command(
            [
                sys.executable,
                "scripts/enterprise_rag_bench/compare_retrieval_runs.py",
                "--questions-file",
                str(args.questions_file),
                "--baseline-retrieval-file",
                str(args.baseline_retrieval_file),
                "--candidate-retrieval-file",
                str(retrieval_output),
                "--output-jsonl",
                str(compare_details),
                "--report",
                str(compare_report),
                "--markdown",
                str(compare_markdown),
                "--limit",
                str(args.limit),
            ]
        )
        comparison = read_json(compare_report)
        results.append(
            {
                "source_type": source_type,
                "decision": decision_for(comparison),
                "retrieval_output": str(retrieval_output),
                "rerank_report": str(rerank_report),
                "comparison_report": str(compare_report),
                "metrics": comparison["metrics"],
                "improved_question_ids": comparison.get("improved_question_ids", []),
                "regressed_question_ids": comparison.get("regressed_question_ids", []),
            }
        )

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.source_route_sweep.v1",
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "route_question_types": args.route_question_types,
        "source_types": args.source_types,
        "results": results,
        "promote_candidates": [
            item["source_type"] for item in results if item["decision"] == "promote_candidate"
        ],
        "rejected_sources": [
            item["source_type"] for item in results if item["decision"] == "reject_regression"
        ],
    }
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    return report


def split_csv(value: str) -> list[str]:
    return [item.strip() for item in value.split(",") if item.strip()]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--doc-views-file", type=Path, required=True)
    parser.add_argument("--embedding-cache", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--run-id", default="source_route_sweep")
    parser.add_argument("--source-types", required=True)
    parser.add_argument("--route-question-types", default="semantic")
    parser.add_argument("--score-candidate-limit", type=int, default=800)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--seed-count", type=int, default=3)
    parser.add_argument("--protect-baseline-prefix", type=int, default=9)
    args = parser.parse_args()
    args.source_types = split_csv(args.source_types)
    args.route_question_types = split_csv(args.route_question_types)
    if not args.source_types:
        raise SystemExit("--source-types must include at least one source type")
    if not args.route_question_types:
        raise SystemExit("--route-question-types must include at least one question type")
    return args


def main() -> None:
    report = run(parse_args())
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
