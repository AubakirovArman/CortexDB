#!/usr/bin/env python3
"""Generate EnterpriseRAG-Bench answers from retrieved CortexDB document IDs."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import threading
import time
from pathlib import Path
from typing import Any

from answer_chat import chat
from answer_context import load_context
from answer_artifacts import evidence_plan_for_row, evidence_table_for_row, maps_by_id
from answer_guard import guard_unsupported_claims
from answer_intent import answer_intent_profile
from answer_prompts import build_prompt
from answer_repair import build_self_consistency_repair_prompt, should_self_consistency_repair
from official_clean import assert_clean_retrieval
from progress_logging import ProgressLogger


DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"
LOGGER = ProgressLogger("answer-runner")


def log(message: str) -> None:
    LOGGER.log(message)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def append_jsonl(path: Path, row: dict[str, Any], lock: threading.Lock) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with lock:
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def high_level_reference_context(path: Path | None, max_chars: int) -> str:
    if path is None or max_chars <= 0 or not path.exists():
        return ""
    text = path.read_text(encoding="utf-8")[:max_chars].strip()
    if not text:
        return ""
    return f"--- High-level reference: {path.name} ---\n{text}"


def resolve_answer_budget(
    *,
    question: str,
    question_type: str,
    args: argparse.Namespace,
) -> dict[str, Any]:
    context_mode = args.context_mode
    top_k_context = args.top_k_context
    max_chars_per_doc = args.max_chars_per_doc
    max_tokens = args.max_tokens
    intent_profile = answer_intent_profile(question)
    answer_intent = str(intent_profile.get("intent") or "default")
    adaptive_budget_applied = False
    if getattr(args, "enable_text_intent_budget", False):
        budget_profile = intent_profile.get("budget_profile")
        if isinstance(budget_profile, dict):
            profile_top_k = budget_profile.get("top_k_context")
            profile_max_chars = budget_profile.get("max_chars_per_doc")
            profile_max_tokens = budget_profile.get("max_tokens")
            profile_context_mode = budget_profile.get("context_mode")
            if isinstance(profile_top_k, int) and profile_top_k > top_k_context:
                top_k_context = profile_top_k
                adaptive_budget_applied = True
            if isinstance(profile_max_chars, int) and profile_max_chars > max_chars_per_doc:
                max_chars_per_doc = profile_max_chars
                adaptive_budget_applied = True
            if isinstance(profile_max_tokens, int) and profile_max_tokens > max_tokens:
                max_tokens = profile_max_tokens
                adaptive_budget_applied = True
            if isinstance(profile_context_mode, str) and profile_context_mode:
                context_mode = profile_context_mode
                adaptive_budget_applied = True
        if answer_intent == "complex_project":
            if args.complex_top_k_context > top_k_context:
                top_k_context = args.complex_top_k_context
                adaptive_budget_applied = True
            if args.complex_max_chars_per_doc > max_chars_per_doc:
                max_chars_per_doc = args.complex_max_chars_per_doc
                adaptive_budget_applied = True
            if args.complex_max_tokens > max_tokens:
                max_tokens = args.complex_max_tokens
                adaptive_budget_applied = True
    high_level_override_applied = question_type == "high_level"
    if high_level_override_applied:
        context_mode = args.high_level_context_mode
        top_k_context = args.high_level_top_k_context
        max_chars_per_doc = args.high_level_max_chars_per_doc
        max_tokens = args.high_level_max_tokens
    return {
        "context_mode": context_mode,
        "top_k_context": top_k_context,
        "max_chars_per_doc": max_chars_per_doc,
        "max_tokens": max_tokens,
        "answer_intent": answer_intent,
        "answer_intent_score": int(intent_profile.get("score", 0) or 0),
        "budget_profile": intent_profile.get("budget_profile"),
        "adaptive_budget_applied": adaptive_budget_applied,
        "high_level_override_applied": high_level_override_applied,
    }


def answer_budget_trace_row(
    row: dict[str, Any],
    saved: dict[str, Any] | None,
    args: argparse.Namespace,
) -> dict[str, Any]:
    qid = str(row.get("question_id") or "")
    doc_ids = [str(item) for item in row.get("document_ids", [])]
    if saved and saved.get("active_top_k_context") is not None:
        used_doc_count = saved.get("used_doc_count")
        return {
            "question_id": qid,
            "answer_intent": saved.get("answer_intent"),
            "answer_intent_score": saved.get("answer_intent_score"),
            "context_mode": saved.get("context_mode"),
            "active_top_k_context": saved.get("active_top_k_context"),
            "selected_result_limit": saved.get("selected_result_limit")
            or saved.get("active_top_k_context"),
            "active_max_chars_per_doc": saved.get("active_max_chars_per_doc"),
            "active_max_tokens": saved.get("active_max_tokens"),
            "retrieved_doc_count": saved.get("retrieved_doc_count", len(doc_ids)),
            "used_doc_count": used_doc_count
            if used_doc_count is not None
            else min(len(doc_ids), int(saved.get("active_top_k_context") or 0)),
            "adaptive_budget_applied": saved.get("adaptive_budget_applied"),
            "high_level_override_applied": saved.get("high_level_override_applied"),
            "budget_profile": saved.get("budget_profile"),
            "trace_source": "answer_row",
        }
    budget = resolve_answer_budget(
        question=str(row.get("question", "")),
        question_type=str(row.get("question_type") or ""),
        args=args,
    )
    top_k_context = int(budget["top_k_context"])
    return {
        "question_id": qid,
        "answer_intent": budget["answer_intent"],
        "answer_intent_score": budget["answer_intent_score"],
        "context_mode": budget["context_mode"],
        "active_top_k_context": top_k_context,
        "selected_result_limit": top_k_context,
        "active_max_chars_per_doc": budget["max_chars_per_doc"],
        "active_max_tokens": budget["max_tokens"],
        "retrieved_doc_count": len(doc_ids),
        "used_doc_count": min(len(doc_ids), top_k_context),
        "adaptive_budget_applied": budget["adaptive_budget_applied"],
        "high_level_override_applied": budget["high_level_override_applied"],
        "budget_profile": budget["budget_profile"],
        "trace_source": "recomputed",
    }


def write_answer_budget_trace(
    rows: list[dict[str, Any]],
    existing: dict[Any, dict[str, Any]],
    args: argparse.Namespace,
    output_budget_trace: Path,
) -> tuple[int, int]:
    trace_rows = [
        answer_budget_trace_row(row, existing.get(row.get("question_id")), args) for row in rows
    ]
    output_budget_trace.write_text(
        "".join(
            json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in trace_rows
        ),
        encoding="utf-8",
    )
    adaptive = sum(1 for row in trace_rows if row.get("adaptive_budget_applied"))
    return len(trace_rows), adaptive


def run(args: argparse.Namespace) -> dict[str, Any]:
    global LOGGER
    LOGGER = ProgressLogger(
        "answer-runner",
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
    log(
        "loaded answer run "
        f"rows={len(rows)} existing={len(existing)} pending={len(pending)} "
        f"workers={args.workers} model={args.model} context_mode={args.context_mode}"
    )
    LOGGER.progress(
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
        log(f"nothing pending; reuse report {output_report}")
        report = read_json(output_report)
        report["budget_trace_file"] = str(output_budget_trace)
        report["budget_trace_questions"] = trace_count
        report["adaptive_budget_questions"] = adaptive_budget_questions
        write_json(output_report, report)
        LOGGER.progress(
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

    def generate(row: dict[str, Any]) -> dict[str, Any]:
        qid = str(row.get("question_id") or "")
        doc_ids = [str(item) for item in row.get("document_ids", [])]
        question = str(row.get("question", ""))
        question_type = str(row.get("question_type") or "")
        budget = resolve_answer_budget(
            question=question,
            question_type=question_type,
            args=args,
        )
        context_mode = str(budget["context_mode"])
        top_k_context = int(budget["top_k_context"])
        max_chars_per_doc = int(budget["max_chars_per_doc"])
        max_tokens = int(budget["max_tokens"])
        answer_intent = str(budget["answer_intent"])
        log(
            "question start answer "
            f"question_id={qid} doc_count={len(doc_ids)} context_mode={context_mode} "
            f"top_k_context={top_k_context} max_tokens={max_tokens} answer_intent={answer_intent}"
        )
        LOGGER.status(
            stage="answer_generation",
            state="running",
            active_step="answer_question",
            active_question_id=qid,
            active_doc_count=len(doc_ids),
            active_context_mode=context_mode,
            active_answer_intent=answer_intent,
            active_top_k_context=top_k_context,
            model=args.model,
            completed_questions=completed_count(),
            total_questions=len(rows),
            pending_questions=max(0, len(rows) - completed_count()),
            prompt_tokens=usage_totals["prompt_tokens"],
            completion_tokens=usage_totals["completion_tokens"],
            total_tokens=usage_totals["total_tokens"],
        )
        try:
            context = load_context(
                doc_ids[:top_k_context],
                uuid_index,
                args.sources_dir,
                max_chars_per_doc,
                question,
                context_mode,
            )
            if question_type == "high_level":
                reference = high_level_reference_context(
                    args.high_level_reference_file,
                    args.high_level_reference_max_chars,
                )
                if reference:
                    context = f"{reference}\n\n{context}" if context else reference
            evidence_plan = evidence_plan_for_row(row, evidence_plans, args.include_evidence_plan)
            evidence_table = evidence_table_for_row(
                row=row,
                tables=evidence_tables,
                include=args.include_evidence_table,
                doc_ids=doc_ids[:top_k_context],
                uuid_index=uuid_index,
                sources_dir=args.sources_dir,
                max_facts_per_doc=args.max_evidence_facts_per_doc,
                max_table_rows=args.max_evidence_table_rows,
            )
            answer, usage, elapsed_ms = chat(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=build_prompt(row, context, args.prompt_style, evidence_plan, evidence_table),
                max_tokens=max_tokens,
                retries=args.retries,
                omit_thinking_field=args.omit_thinking_field,
                gemini_native=args.gemini_native,
                gemini_thinking_budget=args.gemini_thinking_budget,
                openai_reasoning=getattr(args, "openai_reasoning", False),
            )
            answer_guard_result = {"mode": args.unsupported_claim_guard}
            if args.unsupported_claim_guard != "off":
                answer, answer_guard_result = guard_unsupported_claims(
                    answer,
                    context,
                    mode=args.unsupported_claim_guard,
                )
            self_consistency_result: dict[str, Any] = {
                "enabled": args.self_consistency_repair,
                "attempted": False,
            }
            if args.self_consistency_repair:
                _report_answer, consistency_report = guard_unsupported_claims(
                    answer,
                    context,
                    mode="report",
                )
                self_consistency_result["pre_repair_guard"] = consistency_report
                if should_self_consistency_repair(consistency_report):
                    repair_prompt = build_self_consistency_repair_prompt(
                        question=question,
                        context=context,
                        draft_answer=answer,
                        guard_report=consistency_report,
                    )
                    repaired_answer, repair_usage, repair_elapsed_ms = chat(
                        api_key=api_key,
                        base_url=args.base_url,
                        model=args.model,
                        prompt=repair_prompt,
                        max_tokens=max_tokens,
                        retries=args.self_consistency_retries,
                        omit_thinking_field=args.omit_thinking_field,
                        gemini_native=args.gemini_native,
                        gemini_thinking_budget=args.gemini_thinking_budget,
                        openai_reasoning=getattr(args, "openai_reasoning", False),
                    )
                    answer, repair_guard = guard_unsupported_claims(
                        repaired_answer,
                        context,
                        mode="repair",
                    )
                    for key in usage_totals:
                        usage[key] = int(usage.get(key, 0) or 0) + int(
                            repair_usage.get(key, 0) or 0
                        )
                    self_consistency_result.update(
                        {
                            "attempted": True,
                            "elapsed_ms": repair_elapsed_ms,
                            "guard": repair_guard,
                            "prompt_tokens": int(repair_usage.get("prompt_tokens", 0) or 0),
                            "completion_tokens": int(
                                repair_usage.get("completion_tokens", 0) or 0
                            ),
                            "total_tokens": int(repair_usage.get("total_tokens", 0) or 0),
                        }
                    )
        except Exception as error:
            log(f"question failed answer question_id={qid} error={error}")
            LOGGER.status(
                stage="answer_generation",
                state="failed",
                active_step="answer_question",
                active_question_id=qid,
                failed_question_id=qid,
                error=str(error),
                completed_questions=completed_count(),
                total_questions=len(rows),
                pending_questions=max(0, len(rows) - completed_count()),
            )
            raise
        with usage_lock:
            for key in usage_totals:
                usage_totals[key] += int(usage.get(key, 0) or 0)
        log(
            "question done answer "
            f"question_id={qid} elapsed_ms={elapsed_ms} "
            f"prompt_tokens={int(usage.get('prompt_tokens', 0) or 0)} "
            f"completion_tokens={int(usage.get('completion_tokens', 0) or 0)} "
            f"total_tokens={int(usage.get('total_tokens', 0) or 0)}"
        )
        return {
            "question_id": qid,
            "answer": answer,
            "document_ids": doc_ids,
            "elapsed_ms": elapsed_ms,
            "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
            "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
            "total_tokens": int(usage.get("total_tokens", 0) or 0),
            "model": args.model,
            "context_mode": context_mode,
            "answer_intent": answer_intent,
            "answer_intent_score": budget["answer_intent_score"],
            "budget_profile": budget["budget_profile"],
            "active_top_k_context": top_k_context,
            "selected_result_limit": top_k_context,
            "active_max_chars_per_doc": max_chars_per_doc,
            "active_max_tokens": max_tokens,
            "retrieved_doc_count": len(doc_ids),
            "used_doc_count": min(len(doc_ids), top_k_context),
            "adaptive_budget_applied": budget["adaptive_budget_applied"],
            "high_level_override_applied": budget["high_level_override_applied"],
            "answer_guard": answer_guard_result,
            "self_consistency_repair": self_consistency_result,
            "prompt_style": args.prompt_style,
            "evidence_plan": "included" if evidence_plan else "none",
            "evidence_table": "included" if evidence_table else "none",
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(generate, row) for row in pending]
        log(f"queued answer jobs pending={len(pending)} workers={args.workers}")
        LOGGER.status(
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
                LOGGER.progress(
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
                LOGGER.status(
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
    log(f"answer run complete questions={len(ordered)} report={output_report}")
    LOGGER.progress(
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--api-key-file", type=Path, required=True)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--top-k-context", type=int, default=6)
    parser.add_argument("--max-chars-per-doc", type=int, default=1600)
    parser.add_argument("--max-tokens", type=int, default=180)
    parser.add_argument(
        "--enable-text-intent-budget",
        action="store_true",
        help="Use oracle-free question text intent to increase budget for complex project-style answers.",
    )
    parser.add_argument("--complex-top-k-context", type=int, default=10)
    parser.add_argument("--complex-max-chars-per-doc", type=int, default=2600)
    parser.add_argument("--complex-max-tokens", type=int, default=900)
    parser.add_argument(
        "--unsupported-claim-guard",
        choices=["off", "report", "suppress", "repair"],
        default="off",
        help="Report, remove, or repair answer statements whose exact numbers, dates, IDs, versions, or paths are absent from evidence.",
    )
    parser.add_argument(
        "--self-consistency-repair",
        action="store_true",
        help="Run one evidence-only repair call when the draft answer contains unsupported exact markers.",
    )
    parser.add_argument("--self-consistency-retries", type=int, default=1)
    parser.add_argument("--high-level-top-k-context", type=int, default=10)
    parser.add_argument("--high-level-max-chars-per-doc", type=int, default=5000)
    parser.add_argument("--high-level-reference-file", type=Path)
    parser.add_argument("--high-level-reference-max-chars", type=int, default=10000)
    parser.add_argument("--high-level-max-tokens", type=int, default=260)
    parser.add_argument(
        "--high-level-context-mode",
        choices=[
            "leading",
            "evidence-spans",
            "span-plus-fallback",
            "evidence-first",
            "brain-digest",
            "question-window",
            "question-window-digest",
            "question-window-digest-ranked",
        ],
        default="leading",
    )
    parser.add_argument(
        "--prompt-style",
        choices=[
            "baseline",
            "official-clean-v1",
            "fact-focused-v2",
            "evidence-selection-v5",
            "type-aware-v9",
            "type-aware-v13",
            "type-aware-v15",
            "type-aware-v17",
            "evidence-first-v18",
            "evidence-audit-v11",
        ],
        default="baseline",
    )
    parser.add_argument(
        "--context-mode",
        choices=[
            "leading",
            "evidence-spans",
            "span-plus-fallback",
            "evidence-first",
            "brain-digest",
            "question-window",
            "question-window-digest",
            "question-window-digest-ranked",
        ],
        default="leading",
    )
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--progress-every", type=int, default=10)
    parser.add_argument("--evidence-plan-file", type=Path)
    parser.add_argument(
        "--include-evidence-plan",
        action="store_true",
        help="Inject deterministic evidence slots into the answer prompt.",
    )
    parser.add_argument("--evidence-table-file", type=Path)
    parser.add_argument("--max-evidence-facts-per-doc", type=int, default=6)
    parser.add_argument("--max-evidence-table-rows", type=int, default=40)
    parser.add_argument(
        "--include-evidence-table",
        action="store_true",
        help="Inject deterministic evidence fact rows into the answer prompt.",
    )
    parser.add_argument(
        "--omit-thinking-field",
        action="store_true",
        help="Do not send the DeepSeek-specific thinking field; required by some OpenAI-compatible APIs.",
    )
    parser.add_argument("--gemini-native", action="store_true", help="Use Gemini native generateContent API.")
    parser.add_argument("--gemini-thinking-budget", type=int, default=0)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    try:
        print(json.dumps(run(parser.parse_args()), sort_keys=True))
        return 0
    except Exception as error:
        LOGGER.status(stage="answer_generation", state="failed", error=str(error))
        raise


if __name__ == "__main__":
    raise SystemExit(main())
