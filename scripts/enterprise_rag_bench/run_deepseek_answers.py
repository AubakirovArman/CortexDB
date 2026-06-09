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


DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"


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


def run(args: argparse.Namespace) -> dict[str, Any]:
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
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    started = time.perf_counter()
    pending = [row for row in rows if row.get("question_id") not in existing]
    if not pending and output_report.exists():
        return read_json(output_report)

    def generate(row: dict[str, Any]) -> dict[str, Any]:
        doc_ids = [str(item) for item in row.get("document_ids", [])]
        question = str(row.get("question", ""))
        context = load_context(
            doc_ids[: args.top_k_context],
            uuid_index,
            args.sources_dir,
            args.max_chars_per_doc,
            question,
            args.context_mode,
        )
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
            max_tokens=args.max_tokens,
            retries=args.retries,
            omit_thinking_field=args.omit_thinking_field,
            gemini_native=args.gemini_native,
            gemini_thinking_budget=args.gemini_thinking_budget,
        )
        with usage_lock:
            for key in usage_totals:
                usage_totals[key] += int(usage.get(key, 0) or 0)
        return {
            "question_id": row.get("question_id"),
            "answer": answer,
            "document_ids": doc_ids,
            "elapsed_ms": elapsed_ms,
            "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
            "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
            "total_tokens": int(usage.get("total_tokens", 0) or 0),
            "model": args.model,
            "context_mode": args.context_mode,
            "prompt_style": args.prompt_style,
            "evidence_plan": "included" if evidence_plan else "none",
            "evidence_table": "included" if evidence_table else "none",
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(generate, row) for row in pending]
        for future in concurrent.futures.as_completed(futures):
            saved = future.result()
            existing[saved["question_id"]] = saved
            append_jsonl(output_jsonl, saved, output_lock)
            if args.progress_every and len(existing) % args.progress_every == 0:
                print(f"generated {len(existing)}/{len(rows)}")

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
        "--prompt-style",
        choices=[
            "baseline",
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
    print(json.dumps(run(parser.parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
