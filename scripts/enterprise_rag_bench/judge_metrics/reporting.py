from __future__ import annotations

import argparse
import time
from typing import Any

from judge_metrics.io import read_json, write_json
from judge_metrics.stats import aggregate_stats, question_type_stats


def log_loaded_run(
    *,
    args: argparse.Namespace,
    qids: list[str],
    existing: dict[str, dict[str, Any]],
    pending: list[str],
    logger: Any,
) -> None:
    logger.log(
        "loaded judge run "
        f"questions={len(qids)} existing={len(existing)} pending={len(pending)} "
        f"workers={args.workers} model={args.model}"
    )
    logger.progress(
        stage="judging",
        state="running",
        completed=len(existing),
        total=len(qids),
        unit="questions",
        total_questions=len(qids),
        existing_questions=len(existing),
        pending_questions=len(pending),
        completed_questions=len(existing),
        prompt_tokens=0,
        completion_tokens=0,
        total_tokens=0,
        results_file=str(args.results_file),
        judgments_file=str(args.judgments_file),
    )


def reuse_existing_results(
    args: argparse.Namespace,
    qids: list[str],
    existing: dict[str, dict[str, Any]],
    logger: Any,
) -> dict[str, Any]:
    logger.log(f"nothing pending; reuse results {args.results_file}")
    logger.progress(
        stage="judging",
        state="done",
        completed=len(existing),
        total=len(qids),
        unit="questions",
        total_questions=len(qids),
        completed_questions=len(existing),
        pending_questions=0,
        results_file=str(args.results_file),
        judgments_file=str(args.judgments_file),
    )
    return read_json(args.results_file)


def log_question_start(
    args: argparse.Namespace,
    qid: str,
    recall: float | None,
    invalid_extra: int | None,
    completed_count: Any,
    qids: list[str],
    usage_totals: dict[str, int],
    logger: Any,
) -> None:
    logger.log(
        "question start judge "
        f"question_id={qid} doc_recall={recall} invalid_extra_docs={invalid_extra}"
    )
    logger.status(
        stage="judging",
        state="running",
        active_step="judge_question",
        active_question_id=qid,
        active_document_recall_pct=recall,
        active_invalid_extra_docs=invalid_extra,
        model=args.model,
        completed_questions=completed_count(),
        total_questions=len(qids),
        pending_questions=max(0, len(qids) - completed_count()),
        prompt_tokens=usage_totals["prompt_tokens"],
        completion_tokens=usage_totals["completion_tokens"],
        total_tokens=usage_totals["total_tokens"],
    )


def log_question_done(
    qid: str,
    judged: dict[str, Any],
    usage: dict[str, Any],
    elapsed_ms: int,
    logger: Any,
) -> None:
    logger.log(
        "question done judge "
        f"question_id={qid} answer_correct={judged['answer_correct']} "
        f"completeness_pct={judged['completeness_pct']} elapsed_ms={elapsed_ms} "
        f"prompt_tokens={int(usage.get('prompt_tokens', 0) or 0)} "
        f"completion_tokens={int(usage.get('completion_tokens', 0) or 0)} "
        f"total_tokens={int(usage.get('total_tokens', 0) or 0)}"
    )


def log_queued_jobs(
    args: argparse.Namespace,
    qids: list[str],
    pending: list[str],
    completed_count: Any,
    logger: Any,
) -> None:
    logger.log(f"queued judge jobs pending={len(pending)} workers={args.workers}")
    logger.status(
        stage="judging",
        state="running",
        active_step="queued_judge_jobs",
        queued_questions=len(pending),
        workers=args.workers,
        completed_questions=completed_count(),
        total_questions=len(qids),
        pending_questions=max(0, len(qids) - completed_count()),
    )


def log_progress(
    args: argparse.Namespace,
    qids: list[str],
    row: dict[str, Any],
    completed: int,
    usage_totals: dict[str, int],
    started: float,
    logger: Any,
) -> None:
    common = {
        "total_questions": len(qids),
        "completed_questions": completed,
        "pending_questions": max(0, len(qids) - completed),
        "prompt_tokens": usage_totals["prompt_tokens"],
        "completion_tokens": usage_totals["completion_tokens"],
        "total_tokens": usage_totals["total_tokens"],
        "last_question_id": str(row["question_id"]),
    }
    if (args.progress_every and completed % args.progress_every == 0) or completed == len(qids):
        logger.progress(
            stage="judging",
            state="running",
            completed=completed,
            total=len(qids),
            unit="questions",
            **common,
        )
    else:
        logger.status(
            stage="judging",
            state="running",
            elapsed_seconds=round(time.perf_counter() - started, 1),
            **common,
        )


def write_report(
    args: argparse.Namespace,
    qids: list[str],
    existing: dict[str, dict[str, Any]],
    usage_totals: dict[str, int],
    logger: Any,
) -> dict[str, Any]:
    rows = [existing[qid] for qid in qids if qid in existing]
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.local_judge_metrics.v2",
        "judge_model": args.model,
        "judge_provider": "gemini" if args.gemini_native else "openai_compatible",
        "thinking": thinking_label(args),
        "answers_file": str(args.answers_file),
        "questions_file": str(args.questions_file),
        "judgments_file": str(args.judgments_file),
        "questions": rows,
        "aggregate_stats": aggregate_stats(rows),
        "question_type_stats": question_type_stats(rows),
        **usage_totals,
    }
    write_json(args.results_file, report)
    logger.progress(
        stage="judging",
        state="done",
        completed=len(rows),
        total=len(qids),
        unit="questions",
        total_questions=len(qids),
        completed_questions=len(rows),
        pending_questions=max(0, len(qids) - len(rows)),
        prompt_tokens=usage_totals["prompt_tokens"],
        completion_tokens=usage_totals["completion_tokens"],
        total_tokens=usage_totals["total_tokens"],
        results_file=str(args.results_file),
        judgments_file=str(args.judgments_file),
        overall=report["aggregate_stats"].get("combined_correctness_completeness_score"),
    )
    return report


def thinking_label(args: argparse.Namespace) -> str:
    if args.gemini_native:
        return f"gemini_budget_{args.gemini_thinking_budget}"
    return "omitted" if args.omit_thinking_field else "disabled"
