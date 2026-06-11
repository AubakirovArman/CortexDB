#!/usr/bin/env python3
"""Run an official-clean EnterpriseRAG regression and validate its artifacts."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]


def run(cmd: list[str], *, env: dict[str, str] | None = None) -> None:
    print("+ " + " ".join(cmd), flush=True)
    subprocess.run(cmd, cwd=ROOT, env=env, check=True)


def load_json(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: expected JSON object")
    return payload


def provider_target(size: int, answer_provider: str, judge_provider: str) -> str:
    if answer_provider == judge_provider and answer_provider in {"gemma", "gemini", "deepseek"}:
        return f"enterprise-rag-bench-official-clean-{size}-{answer_provider}"
    return f"enterprise-rag-bench-official-clean-{size}"


def make_env(args: argparse.Namespace) -> dict[str, str]:
    env = os.environ.copy()
    env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL"] = args.run_label
    env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE"] = args.stage
    env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER"] = args.answer_provider
    env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER"] = args.judge_provider
    env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE"] = args.retrieval_mode
    env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK"] = args.rerank
    if args.reuse_db:
        env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB"] = "true"
    if args.db_root:
        env["ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT"] = str(args.db_root)
    return env


def run_dir(size: int, label: str) -> Path:
    return ROOT / "target/enterprise-rag-bench/official-clean" / str(size) / label


def run_report_path(size: int, label: str, answer_provider: str) -> Path:
    return run_dir(size, label) / f"answer-{answer_provider}" / "official_clean_run_report.json"


def run_validation(size: int, label: str, answer_provider: str) -> tuple[Path, Path]:
    base = run_dir(size, label)
    answer_root = base / f"answer-{answer_provider}"
    audit_report = base / "oracle_audit_report.json"
    gate_report = base / "official_clean_gate_report.json"
    run_report = answer_root / "official_clean_run_report.json"

    run(
        [
            sys.executable,
            "scripts/enterprise_rag_bench/oracle_usage_audit.py",
            "--clean-questions",
            str(base / "questions.clean.jsonl"),
            "--clean-retrieval",
            str(base / "retrieval.clean.jsonl"),
            "--answers-file",
            str(answer_root / "answers.jsonl"),
            "--report",
            str(audit_report),
        ]
    )
    run(
        [
            sys.executable,
            "scripts/enterprise_rag_bench/official_clean_gate.py",
            "--run-report",
            str(run_report),
            "--report",
            str(gate_report),
            "--expected-split",
            "primary",
            "--require-retrieval",
        ]
    )
    return audit_report, gate_report


def write_summary(
    *,
    size: int,
    label: str,
    answer_provider: str,
    judge_provider: str,
    audit_report: Path,
    gate_report: Path,
) -> Path:
    report_path = run_report_path(size, label, answer_provider)
    report = load_json(report_path)
    summary = report.get("summary", {})
    output = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_regression.v1",
        "size": size,
        "run_label": label,
        "answer_provider": answer_provider,
        "judge_provider": judge_provider,
        "run_report": str(report_path.relative_to(ROOT)),
        "oracle_audit_report": str(audit_report.relative_to(ROOT)),
        "official_clean_gate_report": str(gate_report.relative_to(ROOT)),
        "reuse_db": bool(report.get("reuse_db")),
        "retrieval_mode": report.get("retrieval_mode"),
        "rerank": report.get("rerank"),
        "metrics": {
            "overall": summary.get("judge", {}).get("overall"),
            "answer_correctness_pct": summary.get("judge", {}).get("answer_correctness_pct"),
            "answer_completeness_pct": summary.get("judge", {}).get("answer_completeness_pct"),
            "document_recall_pct": summary.get("judge", {}).get("document_recall_pct"),
            "invalid_extra_docs": summary.get("judge", {}).get("invalid_extra_docs"),
            "answer_tokens": summary.get("answer", {}).get("total_tokens"),
            "judge_tokens": summary.get("judge", {}).get("total_tokens"),
            "documents_indexed_this_run": summary.get("retrieval", {})
            .get("performance", {})
            .get("ingest", {})
            .get("documents_indexed"),
        },
    }
    summary_path = run_dir(size, label) / "regression_summary.json"
    summary_path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(output, indent=2, sort_keys=True))
    return summary_path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, choices=(50, 500), default=50)
    parser.add_argument("--run-label")
    parser.add_argument("--answer-provider", default="gemma", choices=("gemma", "gemini", "deepseek"))
    parser.add_argument("--judge-provider", default="gemma", choices=("gemma", "gemini", "deepseek"))
    parser.add_argument(
        "--retrieval-mode",
        default="engine-aql",
        choices=("cached-lexical", "engine-aql", "engine-keyword", "engine-hybrid"),
    )
    parser.add_argument("--rerank", default="weighted", choices=("none", "weighted"))
    parser.add_argument("--stage", default="all")
    parser.add_argument("--reuse-db", action="store_true")
    parser.add_argument("--db-root", type=Path)
    parser.add_argument("--skip-run", action="store_true")
    args = parser.parse_args()

    if args.run_label is None:
        args.run_label = f"regression-{args.size}-{args.answer_provider}-{args.judge_provider}"

    if not args.skip_run:
        target = provider_target(args.size, args.answer_provider, args.judge_provider)
        run(["make", target], env=make_env(args))

    report = run_report_path(args.size, args.run_label, args.answer_provider)
    if not report.exists():
        raise FileNotFoundError(f"missing run report: {report}")

    audit_report, gate_report = run_validation(args.size, args.run_label, args.answer_provider)
    write_summary(
        size=args.size,
        label=args.run_label,
        answer_provider=args.answer_provider,
        judge_provider=args.judge_provider,
        audit_report=audit_report,
        gate_report=gate_report,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
