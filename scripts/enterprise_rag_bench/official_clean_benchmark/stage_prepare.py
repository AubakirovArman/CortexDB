"""Prepare-stage orchestration."""

from __future__ import annotations

import argparse
from pathlib import Path

from .artifacts import size_limit
from .status import log, run_cmd


def prepare(args: argparse.Namespace, p: dict[str, Path]) -> None:
    log(
        "prepare config "
        f"questions_file={args.questions_file} output_questions={p['clean_questions']} "
        f"limit={args.size}"
    )
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/prepare_official_clean_inputs.py",
        "--questions-file",
        str(args.questions_file),
        "--output-questions",
        str(p["clean_questions"]),
        "--report",
        str(p["prepare_report"]),
        "--limit",
        size_limit(args.size),
        "--log-file",
        str(p["prepare_log"]),
        "--status-file",
        str(p["prepare_status"]),
    ]
    run_cmd(
        cmd,
        label="prepare clean questions",
        child_status=p["prepare_status"],
        artifacts={
            "clean_questions": p["clean_questions"],
            "prepare_report": p["prepare_report"],
            "prepare_log": p["prepare_log"],
        },
    )
