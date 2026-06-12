from __future__ import annotations

import argparse
import concurrent.futures
import threading
import time
from typing import Any

from answer_artifacts import maps_by_id
from official_clean import assert_clean_retrieval

from . import log_state
from .budget import write_answer_budget_trace
from .files import append_jsonl, read_json, read_jsonl, write_json
from .worker import AnswerWorker


def run(args: argparse.Namespace) -> dict[str, Any]:
    log_state.configure(
        log_file=getattr(args, "log_file", None),
        status_file=getattr(args, "status_file", None),
    )
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    rows = read_jsonl(args.retrieval_file)
    if args.limit is not None:
        rows = rows[: args.limit]
    if getattr(args, "strict_clean_input", False):
        assert_clean_retrieval(rows)
    uuid_index = read_json(args.uuid_index)
    output_jsonl = args.output_root / "answers.jsonl"
    output_report = args.output_root / "answer_generation_report.json"
    output_budget_trace = args.output_root / "answer_budget_trace.jsonl"
    existing = {row.get("question_id"): row for row in read_jsonl(output_jsonl)}
    evidence_plans = maps_by_id(args.evidence_plan_file, kind="plan")
    evidence_tables = maps_by_id(args.evidence_table_file, kind="table")
    output_lock = threading.Lock()
    trace_lock = threading.Lock()
    usage_lock = threading.Lock()
    progress_lock = threading.Lock()
    completed_counter = {"value": len(existing)}
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    started = time.perf_counter()
    pending = [row for row in rows if row.get("question_id") not in existing]
    log_state.log(
        "loaded answer run "
        f"rows={len(rows)} existing={len(existing)} pending={len(pending)} "
        f"workers={args.workers} model={args.model} context_mode={args.context_mode}"
    )
    log_state.LOGGER.progress(
        stage="answer_generation",
        state="running",
        completed=len(existing),
        total=len(rows),
        unit="questions",
        total_questions=len(rows),
        existing_questions=len(existing),
        pending_questions=len(pending),
        completed_questions=len(existing),
        prompt_tokens=0,
        completion_tokens=0,
        total_tokens=0,
        answers_file=str(output_jsonl),
        report=str(output_report),
    )
    if not pending and output_report.exists():
        trace_count, adaptive_budget_questions = write_answer_budget_trace(
            rows,
            existing,
            args,
            output_budget_trace,
        )
        log_state.log(f"nothing pending; reuse report {output_report}")
        report = read_json(output_report)
        report["budget_trace_file"] = str(output_budget_trace)
        report["budget_trace_questions"] = trace_count
        report["adaptive_budget_questions"] = adaptive_budget_questions
        write_json(output_report, report)
        log_state.LOGGER.progress(
            stage="answer_generation",
            state="done",
            completed=len(existing),
            total=len(rows),
            unit="questions",
            total_questions=len(rows),
            completed_questions=len(existing),
            pending_questions=0,
            answers_file=str(output_jsonl),
            report=str(output_report),
        )
        return report

    def completed_count() -> int:
        with progress_lock:
            return completed_counter["value"]

    worker = AnswerWorker(
        args=args,
        api_key=api_key,
        uuid_index=uuid_index,
        evidence_plans=evidence_plans,
        evidence_tables=evidence_tables,
        rows_len=len(rows),
        usage_totals=usage_totals,
        usage_lock=usage_lock,
        completed_count=completed_count,
    )

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(worker.generate, row) for row in pending]
        log_state.log(f"queued answer jobs pending={len(pending)} workers={args.workers}")
        log_state.LOGGER.status(
            stage="answer_generation",
            state="running",
            active_step="queued_answer_jobs",
            queued_questions=len(pending),
            workers=args.workers,
            completed_questions=completed_count(),
            total_questions=len(rows),
            pending_questions=max(0, len(rows) - completed_count()),
        )
        for future in concurrent.futures.as_completed(futures):
            saved = future.result()
            existing[saved["question_id"]] = saved
            append_jsonl(output_jsonl, saved, output_lock)
            append_jsonl(
                output_budget_trace,
                {
                    "question_id": saved["question_id"],
                    "answer_intent": saved.get("answer_intent"),
                    "answer_intent_score": saved.get("answer_intent_score"),
                    "context_mode": saved.get("context_mode"),
                    "active_top_k_context": saved.get("active_top_k_context"),
                    "selected_result_limit": saved.get("selected_result_limit"),
                    "active_max_chars_per_doc": saved.get("active_max_chars_per_doc"),
                    "active_max_tokens": saved.get("active_max_tokens"),
                    "retrieved_doc_count": saved.get("retrieved_doc_count"),
                    "used_doc_count": saved.get("used_doc_count"),
                    "adaptive_budget_applied": saved.get("adaptive_budget_applied"),
                    "high_level_override_applied": saved.get("high_level_override_applied"),
                    "budget_profile": saved.get("budget_profile"),
                },
                trace_lock,
            )
            with progress_lock:
                completed_counter["value"] = len(existing)
                completed = completed_counter["value"]
            should_log = (
                (args.progress_every and completed % args.progress_every == 0)
                or completed == len(rows)
            )
            if should_log:
                log_state.LOGGER.progress(
                    stage="answer_generation",
                    state="running",
                    completed=completed,
                    total=len(rows),
                    unit="questions",
                    total_questions=len(rows),
                    completed_questions=completed,
                    pending_questions=max(0, len(rows) - completed),
                    prompt_tokens=usage_totals["prompt_tokens"],
                    completion_tokens=usage_totals["completion_tokens"],
                    total_tokens=usage_totals["total_tokens"],
                    last_question_id=str(saved["question_id"]),
                )
            else:
                log_state.LOGGER.status(
                    stage="answer_generation",
                    state="running",
                    total_questions=len(rows),
                    completed_questions=completed,
                    pending_questions=max(0, len(rows) - completed),
                    prompt_tokens=usage_totals["prompt_tokens"],
                    completion_tokens=usage_totals["completion_tokens"],
                    total_tokens=usage_totals["total_tokens"],
                    elapsed_seconds=round(time.perf_counter() - started, 1),
                    last_question_id=str(saved["question_id"]),
                )

    ordered = [existing[row.get("question_id")] for row in rows if row.get("question_id") in existing]
    budget_trace_questions, adaptive_budget_questions = write_answer_budget_trace(
        rows,
        existing,
        args,
        output_budget_trace,
    )
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.deepseek_answers_report.v1",
        "model": args.model,
        "thinking": (
            f"gemini_budget_{args.gemini_thinking_budget}"
            if args.gemini_native
            else "omitted"
            if args.omit_thinking_field
            else "disabled"
        ),
        "context_mode": args.context_mode,
        "prompt_style": args.prompt_style,
        "include_evidence_plan": args.include_evidence_plan,
        "evidence_plan_file": str(args.evidence_plan_file) if args.evidence_plan_file else None,
        "include_evidence_table": args.include_evidence_table,
        "evidence_table_file": str(args.evidence_table_file) if args.evidence_table_file else None,
        "enable_text_intent_budget": getattr(args, "enable_text_intent_budget", False),
        "complex_top_k_context": getattr(args, "complex_top_k_context", None),
        "complex_max_chars_per_doc": getattr(args, "complex_max_chars_per_doc", None),
        "complex_max_tokens": getattr(args, "complex_max_tokens", None),
        "unsupported_claim_guard": getattr(args, "unsupported_claim_guard", "off"),
        "self_consistency_repair": getattr(args, "self_consistency_repair", False),
        "questions": len(ordered),
        "retrieval_file": str(args.retrieval_file),
        "answers_file": str(output_jsonl),
        "budget_trace_file": str(output_budget_trace),
        "budget_trace_questions": budget_trace_questions,
        "adaptive_budget_questions": adaptive_budget_questions,
        "wall_elapsed_ms": int((time.perf_counter() - started) * 1000),
        **usage_totals,
    }
    write_json(output_report, report)
    log_state.log(f"answer run complete questions={len(ordered)} report={output_report}")
    log_state.LOGGER.progress(
        stage="answer_generation",
        state="done",
        completed=len(ordered),
        total=len(rows),
        unit="questions",
        total_questions=len(rows),
        completed_questions=len(ordered),
        pending_questions=max(0, len(rows) - len(ordered)),
        prompt_tokens=usage_totals["prompt_tokens"],
        completion_tokens=usage_totals["completion_tokens"],
        total_tokens=usage_totals["total_tokens"],
        answers_file=str(output_jsonl),
        report=str(output_report),
    )
    return report
