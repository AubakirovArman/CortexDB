"""Judge-stage orchestration."""

from __future__ import annotations

import argparse
from pathlib import Path

from .artifacts import size_limit
from .status import log, run_cmd


def judge(args: argparse.Namespace, p: dict[str, Path]) -> None:
    log(
        "judge config "
        f"provider={args.judge_provider} workers={args.judge_workers} "
        f"timeout_seconds={args.judge_timeout_seconds} progress_every={args.progress_every}"
    )
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/run_official_clean_judge.py",
        "--answers-file",
        str(p["answers"]),
        "--questions-file",
        str(args.questions_file),
        "--results-file",
        str(p["judge_results"]),
        "--judgments-file",
        str(p["judge_rows"]),
        "--provider",
        args.judge_provider,
        "--workers",
        str(args.judge_workers),
        "--timeout-seconds",
        str(args.judge_timeout_seconds),
        "--limit",
        size_limit(args.size),
        "--progress-every",
        str(args.progress_every),
        "--log-file",
        str(p["judge_log"]),
        "--status-file",
        str(p["judge_status"]),
    ]
    run_cmd(
        cmd,
        label=f"judge answers with {args.judge_provider}",
        child_status=p["judge_status"],
        artifacts={
            "judge_results": p["judge_results"],
            "judge_rows": p["judge_rows"],
            "judge_log": p["judge_log"],
            "judge_status": p["judge_status"],
        },
    )
