#!/usr/bin/env python3
"""Generate EnterpriseRAG-Bench answers in oracle-free mode."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from official_clean import (
    assert_clean_retrieval,
    model_profile,
    read_jsonl,
    require_profile_key,
    write_temp_key,
)
from progress_logging import ProgressLogger
from run_deepseek_answers import run as run_answers


LOGGER = ProgressLogger("official-clean-answer")


def log(message: str) -> None:
    LOGGER.log(message)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument(
        "--provider", choices=["gemma", "gemini", "deepseek", "openai"], required=True
    )
    parser.add_argument("--top-k-context", type=int, default=8)
    parser.add_argument("--max-chars-per-doc", type=int, default=2200)
    parser.add_argument("--max-tokens", type=int, default=420)
    parser.add_argument(
        "--enable-text-intent-budget",
        action="store_true",
        help="Use oracle-free question text intent to increase budget for complex answers.",
    )
    parser.add_argument("--complex-top-k-context", type=int, default=10)
    parser.add_argument("--complex-max-chars-per-doc", type=int, default=2600)
    parser.add_argument("--complex-max-tokens", type=int, default=900)
    parser.add_argument(
        "--unsupported-claim-guard",
        choices=["off", "report", "suppress", "repair"],
        default="off",
        help="Report, remove, or repair answer statements with unsupported exact concrete markers.",
    )
    parser.add_argument(
        "--self-consistency-repair",
        action="store_true",
        help="Run one evidence-only repair call when the draft answer contains unsupported exact markers.",
    )
    parser.add_argument("--self-consistency-retries", type=int, default=1)
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
            "full-doc",
            "single-doc-full",
        ],
        default="question-window-digest-ranked",
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
        default="official-clean-v1",
        help="Answer prompt style; evidence-first-v18 enables the two-phase evidence-first prompt.",
    )
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--progress-every", type=int, default=10)
    parser.add_argument("--evidence-table-file", type=Path)
    parser.add_argument("--evidence-plan-file", type=Path)
    parser.add_argument(
        "--include-evidence-plan",
        action="store_true",
        help="Inject deterministic oracle-free evidence slot/completeness plans into the prompt.",
    )
    parser.add_argument("--max-evidence-facts-per-doc", type=int, default=6)
    parser.add_argument("--max-evidence-table-rows", type=int, default=40)
    parser.add_argument(
        "--include-evidence-table",
        action="store_true",
        help="Inject deterministic evidence fact rows into the official-clean prompt.",
    )
    parser.add_argument("--gemini-thinking-budget", type=int, default=0)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    global LOGGER
    LOGGER = ProgressLogger(
        "official-clean-answer",
        log_file=args.log_file,
        status_file=args.status_file,
    )

    rows = read_jsonl(args.retrieval_file)
    pending_limit = args.limit if args.limit is not None else len(rows)
    log(
        "loaded retrieval "
        f"rows={len(rows)} limit={pending_limit} file={args.retrieval_file}"
    )
    LOGGER.status(
        stage="answer_wrapper",
        state="running",
        step=1,
        total_steps=3,
        retrieval_rows=len(rows),
        limit=pending_limit,
        retrieval_file=str(args.retrieval_file),
    )
    assert_clean_retrieval(rows)
    log("clean retrieval guard passed")
    profile = model_profile(args.provider)
    require_profile_key(profile)
    log(
        "selected provider="
        f"{args.provider} model={profile['model']} output_root={args.output_root}"
    )
    LOGGER.status(
        stage="answer_wrapper",
        state="running",
        step=2,
        total_steps=3,
        provider=args.provider,
        model=str(profile["model"]),
        output_root=str(args.output_root),
    )
    key_file = write_temp_key(args.provider, str(profile["api_key"]))

    log("begin answer generation")
    try:
        report = run_answers(
            SimpleNamespace(
                retrieval_file=args.retrieval_file,
                uuid_index=args.uuid_index,
                sources_dir=args.sources_dir,
                output_root=args.output_root,
                api_key_file=key_file,
                base_url=profile["base_url"],
                model=profile["model"],
                top_k_context=args.top_k_context,
                max_chars_per_doc=args.max_chars_per_doc,
                max_tokens=args.max_tokens,
                enable_text_intent_budget=args.enable_text_intent_budget,
                complex_top_k_context=args.complex_top_k_context,
                complex_max_chars_per_doc=args.complex_max_chars_per_doc,
                complex_max_tokens=args.complex_max_tokens,
                unsupported_claim_guard=args.unsupported_claim_guard,
                self_consistency_repair=args.self_consistency_repair,
                self_consistency_retries=args.self_consistency_retries,
                high_level_top_k_context=args.top_k_context,
                high_level_max_chars_per_doc=args.max_chars_per_doc,
                high_level_reference_file=None,
                high_level_reference_max_chars=0,
                high_level_max_tokens=args.max_tokens,
                high_level_context_mode=args.context_mode,
                prompt_style=args.prompt_style,
                context_mode=args.context_mode,
                workers=args.workers,
                retries=args.retries,
                limit=args.limit,
                progress_every=args.progress_every,
                evidence_plan_file=args.evidence_plan_file,
                include_evidence_plan=args.include_evidence_plan,
                evidence_table_file=args.evidence_table_file,
                max_evidence_facts_per_doc=args.max_evidence_facts_per_doc,
                max_evidence_table_rows=args.max_evidence_table_rows,
                include_evidence_table=args.include_evidence_table,
                omit_thinking_field=bool(profile["omit_thinking_field"]),
                gemini_native=bool(profile["gemini_native"]),
                gemini_thinking_budget=args.gemini_thinking_budget,
                openai_reasoning=bool(profile.get("openai_reasoning", False)),
                strict_clean_input=True,
                log_file=args.log_file,
                status_file=args.status_file,
            )
        )
    except Exception as error:
        LOGGER.status(stage="answer_wrapper", state="failed", error=str(error))
        raise
    log(
        "done answer generation "
        f"questions={report.get('questions')} prompt_tokens={report.get('prompt_tokens')} "
        f"completion_tokens={report.get('completion_tokens')} total_tokens={report.get('total_tokens')}"
    )
    log(f"wrote answers {args.output_root / 'answers.jsonl'}")
    log(f"wrote answer report {args.output_root / 'answer_generation_report.json'}")
    LOGGER.status(
        stage="answer_wrapper",
        state="done",
        step=3,
        total_steps=3,
        questions=report.get("questions"),
        prompt_tokens=report.get("prompt_tokens"),
        completion_tokens=report.get("completion_tokens"),
        total_tokens=report.get("total_tokens"),
        answers_file=str(args.output_root / "answers.jsonl"),
        report=str(args.output_root / "answer_generation_report.json"),
    )
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
