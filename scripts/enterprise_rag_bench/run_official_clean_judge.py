#!/usr/bin/env python3
"""Judge oracle-free EnterpriseRAG-Bench answers with a chosen local model."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from official_clean import model_profile, require_profile_key, write_temp_key
from progress_logging import ProgressLogger
from run_deepseek_answer_metrics import run as run_judge


LOGGER = ProgressLogger("official-clean-judge")


def log(message: str) -> None:
    LOGGER.log(message)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--answers-file", type=Path, required=True)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--results-file", type=Path, required=True)
    parser.add_argument("--judgments-file", type=Path, required=True)
    parser.add_argument(
        "--provider", choices=["gemma", "gemini", "deepseek", "openai"], required=True
    )
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--timeout-seconds", type=float, default=180.0)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--progress-every", type=int, default=10)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    global LOGGER
    LOGGER = ProgressLogger(
        "official-clean-judge",
        log_file=args.log_file,
        status_file=args.status_file,
    )

    profile = model_profile(args.provider)
    require_profile_key(profile)
    log(
        "selected provider="
        f"{args.provider} model={profile['model']} answers={args.answers_file} "
        f"limit={args.limit} results={args.results_file}"
    )
    LOGGER.status(
        stage="judge_wrapper",
        state="running",
        step=1,
        total_steps=2,
        provider=args.provider,
        model=str(profile["model"]),
        answers_file=str(args.answers_file),
        results_file=str(args.results_file),
    )
    key_file = write_temp_key(args.provider, str(profile["api_key"]))
    log("begin judging")
    try:
        report = run_judge(
            SimpleNamespace(
                answers_file=args.answers_file,
                questions_file=args.questions_file,
                results_file=args.results_file,
                judgments_file=args.judgments_file,
                api_key_file=key_file,
                base_url=profile["base_url"],
                model=profile["model"],
                limit=args.limit,
                workers=args.workers,
                retries=args.retries,
                timeout_seconds=args.timeout_seconds,
                progress_every=args.progress_every,
                omit_thinking_field=bool(profile["omit_thinking_field"]),
                gemini_native=bool(profile["gemini_native"]),
                gemini_thinking_budget=0,
                openai_reasoning=bool(profile.get("openai_reasoning", False)),
                log_file=args.log_file,
                status_file=args.status_file,
            )
        )
    except Exception as error:
        LOGGER.status(stage="judge_wrapper", state="failed", error=str(error))
        raise
    stats = report.get("aggregate_stats", {})
    log(
        "done judging "
        f"overall={stats.get('combined_correctness_completeness_score')} "
        f"correctness={stats.get('average_correctness_pct')} "
        f"completeness={stats.get('average_completeness_pct')} "
        f"prompt_tokens={report.get('prompt_tokens')} "
        f"completion_tokens={report.get('completion_tokens')} "
        f"total_tokens={report.get('total_tokens')}"
    )
    log(f"wrote judgments {args.judgments_file}")
    log(f"wrote judge results {args.results_file}")
    LOGGER.status(
        stage="judge_wrapper",
        state="done",
        step=2,
        total_steps=2,
        overall=stats.get("combined_correctness_completeness_score"),
        correctness=stats.get("average_correctness_pct"),
        completeness=stats.get("average_completeness_pct"),
        prompt_tokens=report.get("prompt_tokens"),
        completion_tokens=report.get("completion_tokens"),
        total_tokens=report.get("total_tokens"),
        judgments_file=str(args.judgments_file),
        results_file=str(args.results_file),
    )
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
