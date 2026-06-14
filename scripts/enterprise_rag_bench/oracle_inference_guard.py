#!/usr/bin/env python3
"""Static no-oracle guard for the EnterpriseRAG-Bench official-clean inference path.

Oracle fields are benchmark gold metadata
(`question_type`, `source_types`, `expected_doc_ids`, `gold_answer`,
`answer_facts`). A real deployed system never has them at inference; reading any
of them during retrieval or answer generation is cheating and destroys
generalization to unseen questions. They are legitimate only at the judge/scoring
stage.

This gate fails if any module on the official-clean *inference* path reads an
oracle field off a row (e.g. `row.get("question_type")` or `row["source_types"]`).
It is intentionally scoped to the modules the official-clean profile executes, so
it stays green and meaningful. Cleaning/auditing helpers and tests legitimately
reference oracle fields and are not scanned.

Known non-official caveat: `answer_prompts.py` hosts alternative `type_aware_*`
prompt styles that branch on `question_type`. They are NOT on the official-clean
path (which uses `official-clean-v1`). They must never be selected for a
submission run; this gate guards the path, and the runner pins the clean style.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ORACLE_FIELDS = (
    "question_type",
    "source_types",
    "expected_doc_ids",
    "gold_answer",
    "answer_facts",
)

# Modules executed by the official-clean inference profile that MUST be oracle-free.
INFERENCE_TARGETS = (
    "deepseek_answers_lib",  # answer worker/budget/runner
    "official_clean_benchmark",  # prepare/retrieve/answer/judge stages
    "run_official_clean_answers.py",
    "rerank_with_embeddings.py",
    "answer_context.py",
    "answer_guard.py",
    "answer_chat.py",
    "answer_artifacts.py",
    "answer_repair.py",
    "answer_intent.py",
)

# Oracle field read off a dict/row: `.get("field"` or `["field"]` / `['field']`.
_FIELD_ALT = "|".join(re.escape(field) for field in ORACLE_FIELDS)
_ACCESS_RE = re.compile(
    rf"""\.get\(\s*["'](?:{_FIELD_ALT})["']"""
    rf"""|\[\s*["'](?:{_FIELD_ALT})["']\s*\]"""
)


def python_files(base: Path) -> list[Path]:
    files: list[Path] = []
    for target in INFERENCE_TARGETS:
        path = base / target
        if path.is_dir():
            files.extend(sorted(p for p in path.rglob("*.py") if "__pycache__" not in p.parts))
        elif path.is_file():
            files.append(path)
    return files


def scan_file(path: Path) -> list[tuple[int, str]]:
    violations: list[tuple[int, str]] = []
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if _ACCESS_RE.search(line):
            violations.append((lineno, line.strip()))
    return violations


def main() -> int:
    base = Path(__file__).resolve().parent
    files = python_files(base)
    if not files:
        print("oracle-inference-guard: no inference modules found", file=sys.stderr)
        return 1

    all_violations: list[tuple[Path, int, str]] = []
    for path in files:
        for lineno, text in scan_file(path):
            all_violations.append((path, lineno, text))

    if all_violations:
        print("oracle-inference-guard: FAILED — oracle field read on inference path")
        for path, lineno, text in all_violations:
            rel = path.relative_to(base.parent.parent)
            print(f"  {rel}:{lineno}: {text}")
        print(
            "\nOracle fields must not be read at inference (retrieval/answer). "
            "Move any gold-dependent logic to the judge/scoring stage."
        )
        return 1

    print(
        f"oracle-inference-guard: ok — {len(files)} inference modules scanned, "
        f"no oracle field reads ({', '.join(ORACLE_FIELDS)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
