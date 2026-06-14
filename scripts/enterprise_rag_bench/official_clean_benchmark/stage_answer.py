"""Answer-stage orchestration."""

from __future__ import annotations

import argparse
from pathlib import Path

from .status import log, run_cmd


def answer(args: argparse.Namespace, p: dict[str, Path]) -> None:
    log(
        "answer config "
        f"provider={args.answer_provider} context_mode={args.context_mode} "
        f"prompt_style={args.prompt_style} top_k_context={args.top_k_context} "
        f"max_chars_per_doc={args.max_chars_per_doc} max_tokens={args.max_tokens} "
        f"workers={args.answer_workers} progress_every={args.progress_every} "
        f"include_evidence_plan={args.include_evidence_plan} "
        f"include_evidence_table={args.include_evidence_table} "
        f"gemini_thinking_budget={args.gemini_thinking_budget}"
    )
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/run_official_clean_answers.py",
        "--retrieval-file",
        str(p["clean_retrieval"]),
        "--uuid-index",
        str(args.uuid_index),
        "--sources-dir",
        str(args.sources_dir),
        "--output-root",
        str(p["answer_root"]),
        "--provider",
        args.answer_provider,
        "--top-k-context",
        str(args.top_k_context),
        "--max-chars-per-doc",
        str(args.max_chars_per_doc),
        "--max-tokens",
        str(args.max_tokens),
        "--prompt-style",
        args.prompt_style,
        "--gemini-thinking-budget",
        str(args.gemini_thinking_budget),
        "--unsupported-claim-guard",
        args.unsupported_claim_guard,
        "--context-mode",
        args.context_mode,
        "--workers",
        str(args.answer_workers),
        "--progress-every",
        str(args.progress_every),
        "--log-file",
        str(p["answer_log"]),
        "--status-file",
        str(p["answer_status"]),
    ]
    if args.enable_text_intent_budget:
        cmd.extend(
            [
                "--enable-text-intent-budget",
                "--complex-top-k-context",
                str(args.complex_top_k_context),
                "--complex-max-chars-per-doc",
                str(args.complex_max_chars_per_doc),
                "--complex-max-tokens",
                str(args.complex_max_tokens),
            ]
        )
    if args.self_consistency_repair:
        cmd.extend(
            [
                "--self-consistency-repair",
                "--self-consistency-retries",
                str(args.self_consistency_retries),
            ]
        )
    if args.include_evidence_table:
        cmd.extend(
            [
                "--include-evidence-table",
                "--max-evidence-facts-per-doc",
                str(args.max_evidence_facts_per_doc),
                "--max-evidence-table-rows",
                str(args.max_evidence_table_rows),
            ]
        )
        if args.evidence_table_file:
            cmd.extend(["--evidence-table-file", str(args.evidence_table_file)])
    if args.include_evidence_plan:
        cmd.append("--include-evidence-plan")
        if args.evidence_plan_file:
            cmd.extend(["--evidence-plan-file", str(args.evidence_plan_file)])
    run_cmd(
        cmd,
        label=f"answer questions with {args.answer_provider}",
        child_status=p["answer_status"],
        artifacts={
            "answers": p["answers"],
            "answer_log": p["answer_log"],
            "answer_status": p["answer_status"],
        },
    )
