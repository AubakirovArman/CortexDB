"""Argument parser for official-clean benchmark runs."""

from __future__ import annotations

import argparse
from pathlib import Path

from .constants import BENCH_ROOT, ROOT


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run the oracle-free EnterpriseRAG-Bench pipeline end to end."
    )
    parser.add_argument("--size", type=int, required=True)
    parser.add_argument(
        "--run-label",
        help="Optional artifact namespace under official-clean/<size>/ for mode comparisons.",
    )
    parser.add_argument("--answer-provider", choices=["gemma", "gemini", "deepseek"], required=True)
    parser.add_argument("--judge-provider", choices=["gemma", "gemini", "deepseek"], required=True)
    parser.add_argument("--questions-file", type=Path, default=BENCH_ROOT / "questions.jsonl")
    parser.add_argument(
        "--split-name",
        default="primary",
        help="Logical benchmark split name recorded in reports, for example primary or heldout.",
    )
    parser.add_argument("--uuid-index", type=Path, default=BENCH_ROOT / "generated_data/uuid_index.json")
    parser.add_argument("--sources-dir", type=Path, default=BENCH_ROOT / "generated_data/sources")
    parser.add_argument(
        "--db-root",
        type=Path,
        help="Existing CortexDB corpus root to use with --reuse-db, or target root for ingest.",
    )
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=1000)
    parser.add_argument(
        "--max-documents",
        type=int,
        help="Limit corpus ingest for smoke runs; omit for honest full-corpus evaluation.",
    )
    parser.add_argument(
        "--retrieval-mode",
        choices=[
            "cached-lexical",
            "engine-aql",
            "engine-keyword",
            "engine-hybrid",
            "engine-hybrid-rerank",
        ],
        default="engine-aql",
    )
    parser.add_argument(
        "--rerank",
        choices=["none", "weighted"],
        default="none",
        help="Engine rerank stage applied to engine-keyword/engine-hybrid candidates.",
    )
    parser.add_argument(
        "--embedding-rerank",
        action="store_true",
        help="Rerank a wide candidate set with an external embedding model (bge-m3).",
    )
    parser.add_argument(
        "--embedding-rerank-candidates",
        type=int,
        default=50,
        help="Candidate pool size retrieved before embedding rerank narrows to --top-k.",
    )
    parser.add_argument(
        "--env-file",
        type=Path,
        default=ROOT / ".env",
        help="Env file with CORTEXDB_EMBEDDING_URL/MODEL/API_KEY for embedding rerank.",
    )
    parser.add_argument(
        "--query-vectors",
        type=Path,
        help="JSONL with {question_id, vector}; required for engine-hybrid.",
    )
    parser.add_argument(
        "--document-vectors",
        type=Path,
        help="JSONL with {doc_id, vector}; required for dense engine-hybrid corpus ingest.",
    )
    parser.add_argument(
        "--prefilter-retrieval",
        type=Path,
        help="Official-clean retrieval JSONL used as a reusable candidate source before fallback retrieval.",
    )
    parser.add_argument(
        "--disable-search-prefilter",
        action="store_true",
        help="Force direct engine search instead of the checkpoint/ingest lexical prefilter.",
    )
    parser.add_argument("--top-k-context", type=int, default=8)
    parser.add_argument("--max-chars-per-doc", type=int, default=2200)
    parser.add_argument("--max-tokens", type=int, default=420)
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
        help="How to build context from each retrieved document.",
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
            "exact-first-v1",
        ],
        default="official-clean-v1",
        help="Answer prompt style; evidence-first-v18 enables the two-phase evidence-first prompt.",
    )
    parser.add_argument("--gemini-thinking-budget", type=int, default=0)
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
        "--enable-text-intent-budget",
        action="store_true",
        help="Use oracle-free question text intent to increase budget for complex answers.",
    )
    parser.add_argument("--complex-top-k-context", type=int, default=10)
    parser.add_argument("--complex-max-chars-per-doc", type=int, default=2600)
    parser.add_argument("--complex-max-tokens", type=int, default=900)
    parser.add_argument("--evidence-table-file", type=Path)
    parser.add_argument("--evidence-plan-file", type=Path)
    parser.add_argument(
        "--include-evidence-plan",
        action="store_true",
        help="Inject deterministic oracle-free evidence slot/completeness plans into answer prompts.",
    )
    parser.add_argument("--max-evidence-facts-per-doc", type=int, default=6)
    parser.add_argument("--max-evidence-table-rows", type=int, default=40)
    parser.add_argument(
        "--include-evidence-table",
        action="store_true",
        help="Inject deterministic evidence fact rows into answer prompts.",
    )
    parser.add_argument("--answer-workers", type=int, default=2)
    parser.add_argument("--judge-workers", type=int, default=2)
    parser.add_argument("--judge-timeout-seconds", type=float, default=180.0)
    parser.add_argument("--progress-every", type=int, default=10)
    parser.add_argument("--retrieval-progress-every", type=int, default=50_000)
    parser.add_argument(
        "--reuse-db",
        action="store_true",
        help="Reuse an existing indexed CortexDB root and skip corpus ingest.",
    )
    parser.add_argument(
        "--skip-checkpoint",
        action="store_true",
        help="Skip checkpoint after ingest for one-shot retrieval quality runs.",
    )
    parser.add_argument(
        "--stage",
        choices=["all", "retrieval", "prepare", "retrieve", "answer", "judge"],
        default="all",
    )
    return parser


def validate_args(parser: argparse.ArgumentParser, args: argparse.Namespace) -> None:
    if args.size <= 0:
        parser.error("--size must be positive")
    if args.retrieval_mode in {"engine-hybrid", "engine-hybrid-rerank"} and not args.query_vectors:
        parser.error(f"--retrieval-mode {args.retrieval_mode} requires --query-vectors")
    if args.rerank == "weighted" and args.retrieval_mode == "cached-lexical":
        parser.error(
            "--rerank weighted requires an engine retrieval mode, not cached-lexical"
        )
    if args.embedding_rerank and args.embedding_rerank_candidates <= args.top_k:
        parser.error("--embedding-rerank-candidates must exceed --top-k to have any effect")
    if args.max_documents is not None and args.max_documents <= 0:
        parser.error("--max-documents must be positive")
    if args.run_label and (
        "/" in args.run_label or "\\" in args.run_label or args.run_label in {".", ".."}
    ):
        parser.error("--run-label must be a simple path segment")
    if not args.split_name.strip():
        parser.error("--split-name must not be empty")
