from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from answer_intent import answer_intent_profile


def resolve_answer_budget(
    *,
    question: str,
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
    # Oracle-free: the benchmark `question_type` is gold metadata and must never
    # drive inference. High-level questions are handled oracle-free via retrieval
    # routing (P0-5 company-scope route), not by reading the question type.
    high_level_override_applied = False
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
