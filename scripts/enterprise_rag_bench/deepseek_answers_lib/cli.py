from __future__ import annotations

import argparse
import json
from pathlib import Path

from . import log_state
from .constants import DEFAULT_BASE_URL, DEFAULT_MODEL
from .runner import run


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate EnterpriseRAG-Bench answers from retrieved CortexDB document IDs.")
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
            "full-doc",
            "single-doc-full",
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
            "full-doc",
            "single-doc-full",
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
    parser.add_argument(
        "--gemini-native",
        action="store_true",
        help="Use Gemini native generateContent API.",
    )
    parser.add_argument("--gemini-thinking-budget", type=int, default=0)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    return parser.parse_args()


def main() -> int:
    try:
        print(json.dumps(run(parse_args()), sort_keys=True))
        return 0
    except Exception as error:
        log_state.LOGGER.status(stage="answer_generation", state="failed", error=str(error))
        raise
