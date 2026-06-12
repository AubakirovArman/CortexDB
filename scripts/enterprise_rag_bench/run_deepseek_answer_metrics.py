#!/usr/bin/env python3
"""Score EnterpriseRAG-Bench answers with DeepSeek or compatible judges."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import threading
import time
from pathlib import Path
from typing import Any

from judge_metrics.clients import chat_json
from judge_metrics.io import append_jsonl, by_question_id, read_jsonl
from judge_metrics.prompts import build_prompt
from judge_metrics.reporting import (
    log_loaded_run,
    log_progress,
    log_question_done,
    log_question_start,
    log_queued_jobs,
    reuse_existing_results,
    write_report,
)
from judge_metrics.stats import document_metrics
from progress_logging import ProgressLogger

DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"
LOGGER = ProgressLogger("judge-runner")


def log(message: str) -> None:
    LOGGER.log(message)


def run(args: argparse.Namespace) -> dict[str, Any]:
    global LOGGER
    LOGGER = ProgressLogger(
        "judge-runner",
        log_file=getattr(args, "log_file", None),
        status_file=getattr(args, "status_file", None),
    )
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    questions = by_question_id(read_jsonl(args.questions_file))
    answers = by_question_id(read_jsonl(args.answers_file))
    qids = list(questions.keys())[: args.limit]
    existing = by_question_id(read_jsonl(args.judgments_file))
    output_lock = threading.Lock()
    usage_lock = threading.Lock()
    progress_lock = threading.Lock()
    completed_counter = {"value": len(existing)}
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    started = time.perf_counter()
    pending = [qid for qid in qids if qid not in existing]
    log_loaded_run(args=args, qids=qids, existing=existing, pending=pending, logger=LOGGER)
    if not pending and args.results_file.exists():
        return reuse_existing_results(args, qids, existing, LOGGER)

    def completed_count() -> int:
        with progress_lock:
            return completed_counter["value"]

    def judge(qid: str) -> dict[str, Any]:
        question = questions[qid]
        answer = answers.get(qid, {"question_id": qid, "answer": "", "document_ids": []})
        recall, invalid_extra = document_metrics(question, answer)
        log_question_start(
            args,
            qid,
            recall,
            invalid_extra,
            completed_count,
            qids,
            usage_totals,
            LOGGER,
        )
        judged, usage, elapsed_ms = request_judgment(
            args,
            api_key,
            question,
            answer,
            qid,
            completed_count,
            qids,
        )
        with usage_lock:
            for key in usage_totals:
                usage_totals[key] += int(usage.get(key, 0) or 0)
        log_question_done(qid, judged, usage, elapsed_ms, LOGGER)
        return judgment_row(qid, question, judged, recall, invalid_extra, usage, elapsed_ms)

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(judge, qid) for qid in pending]
        log_queued_jobs(args, qids, pending, completed_count, LOGGER)
        for future in concurrent.futures.as_completed(futures):
            row = future.result()
            existing[row["question_id"]] = row
            append_jsonl(args.judgments_file, row, output_lock)
            with progress_lock:
                completed_counter["value"] = len(existing)
                completed = completed_counter["value"]
            log_progress(args, qids, row, completed, usage_totals, started, LOGGER)

    return write_report(args, qids, existing, usage_totals, LOGGER)


def request_judgment(
    args: argparse.Namespace,
    api_key: str,
    question: dict[str, Any],
    answer: dict[str, Any],
    qid: str,
    completed_count: Any,
    qids: list[str],
) -> tuple[dict[str, Any], dict[str, Any], int]:
    try:
        return chat_json(
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            prompt=build_prompt(question, answer),
            timeout=args.timeout_seconds,
            retries=args.retries,
            omit_thinking_field=args.omit_thinking_field,
            gemini_native=args.gemini_native,
            gemini_thinking_budget=args.gemini_thinking_budget,
            openai_reasoning=getattr(args, "openai_reasoning", False),
        )
    except Exception as error:
        log(f"question failed judge question_id={qid} error={error}")
        LOGGER.status(
            stage="judging",
            state="failed",
            active_step="judge_question",
            active_question_id=qid,
            failed_question_id=qid,
            error=str(error),
            completed_questions=completed_count(),
            total_questions=len(qids),
            pending_questions=max(0, len(qids) - completed_count()),
        )
        raise


def judgment_row(
    qid: str,
    question: dict[str, Any],
    judged: dict[str, Any],
    recall: float | None,
    invalid_extra: int | None,
    usage: dict[str, Any],
    elapsed_ms: int,
) -> dict[str, Any]:
    return {
        "question_id": qid,
        "question_type": question.get("question_type"),
        "answer_correct": judged["answer_correct"],
        "completeness_pct": judged["completeness_pct"],
        "correctness_reasoning": judged["correctness_reasoning"],
        "document_recall_pct": recall,
        "invalid_extra_docs": invalid_extra,
        "corrected": False,
        "elapsed_ms": elapsed_ms,
        "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
        "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
        "total_tokens": int(usage.get("total_tokens", 0) or 0),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--answers-file", type=Path, required=True)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--results-file", type=Path, required=True)
    parser.add_argument("--judgments-file", type=Path, required=True)
    parser.add_argument("--api-key-file", type=Path, required=True)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--timeout-seconds", type=float, default=180.0)
    parser.add_argument("--progress-every", type=int, default=10)
    parser.add_argument(
        "--omit-thinking-field",
        action="store_true",
        help="Do not send the DeepSeek-specific thinking field.",
    )
    parser.add_argument("--gemini-native", action="store_true", help="Use Gemini native API.")
    parser.add_argument("--gemini-thinking-budget", type=int, default=0)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    if args.workers <= 0:
        parser.error("--workers must be positive")
    return args


def main() -> int:
    try:
        print(json.dumps(run(parse_args()), sort_keys=True))
        return 0
    except Exception as error:
        LOGGER.status(stage="judging", state="failed", error=str(error))
        raise


if __name__ == "__main__":
    raise SystemExit(main())
