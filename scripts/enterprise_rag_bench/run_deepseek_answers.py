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
from answer_prompts import build_prompt
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
    uuid_index = read_json(args.uuid_index)
    output_jsonl = args.output_root / "answers.jsonl"
    output_report = args.output_root / "answer_generation_report.json"
    existing = {row.get("question_id"): row for row in read_jsonl(output_jsonl)}
    evidence_plans = maps_by_id(args.evidence_plan_file, kind="plan")
    evidence_tables = maps_by_id(args.evidence_table_file, kind="table")
    output_lock = threading.Lock()
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
        log(f"nothing pending; reuse report {output_report}")
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
        return read_json(output_report)

    def completed_count() -> int:
        with progress_lock:
            return completed_counter["value"]

    def generate(row: dict[str, Any]) -> dict[str, Any]:
        qid = str(row.get("question_id") or "")
        doc_ids = [str(item) for item in row.get("document_ids", [])]
        question = str(row.get("question", ""))
        question_type = str(row.get("question_type") or "")
        context_mode = args.context_mode
        top_k_context = args.top_k_context
        max_chars_per_doc = args.max_chars_per_doc
        max_tokens = args.max_tokens
        if question_type == "high_level":
            context_mode = args.high_level_context_mode
            top_k_context = args.high_level_top_k_context
            max_chars_per_doc = args.high_level_max_chars_per_doc
            max_tokens = args.high_level_max_tokens
        log(
            "question start answer "
            f"question_id={qid} doc_count={len(doc_ids)} context_mode={context_mode} "
            f"top_k_context={top_k_context} max_tokens={max_tokens}"
        )
        LOGGER.status(
            stage="answer_generation",
            state="running",
            active_step="answer_question",
            active_question_id=qid,
            active_doc_count=len(doc_ids),
            active_context_mode=context_mode,
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
                doc_ids=doc_ids[: args.top_k_context],
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
        "questions": len(ordered),
        "retrieval_file": str(args.retrieval_file),
        "answers_file": str(output_jsonl),
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
