#!/usr/bin/env python3
"""Small memory-aware chat agent example backed by CortexDB."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "common"))
from cortex_cli import remember_memory, reset_fixture_database, retrieve_context, verify_fact


DEFAULT_DB = Path("target/live-integration-examples/memory-chat-agent/db")
DEFAULT_SCOPE = "project:investments"
DEFAULT_PREFERENCE = "Prefer cited budget evidence before final answers"
DEFAULT_QUESTION = "What should I say about the Solar Plant budget?"
DEFAULT_FACT = "Solar Plant budget is 1.2B KZT"


class MockMemoryChatModel:
    def answer(
        self,
        question: str,
        memory: dict[str, Any],
        context_pack: dict[str, Any],
        verification: dict[str, Any],
    ) -> str:
        return (
            f"{question} Use the remembered preference cell {memory.get('cell_id')} "
            f"and cite evidence. VERIFY returned {verification.get('verdict')} "
            f"with {context_pack.get('visible_conflict_count')} visible conflict."
        )


def run_demo(
    db_path: Path,
    scope: str,
    question: str,
    preference: str,
    fact: str,
    reset: bool,
) -> dict[str, Any]:
    if reset:
        reset_fixture_database(db_path)
    memory = remember_memory(db_path, scope, preference, memory_type="preference", ttl_seconds=3600)
    context_pack = retrieve_context(db_path, scope, f"{preference}. {question}", limit=10)
    verification = verify_fact(db_path, scope, fact)
    answer = MockMemoryChatModel().answer(question, memory, context_pack, verification)
    return {
        "schema_version": "cortexdb.integration.memory_chat_agent.v1",
        "memory_cell_id": memory.get("cell_id"),
        "ttl_seconds": memory.get("ttl_seconds"),
        "context_cells": len(context_pack.get("cells", [])),
        "visible_conflict_count": context_pack.get("visible_conflict_count"),
        "verify_verdict": verification.get("verdict"),
        "answer": answer,
    }


def assert_self_test(summary: dict[str, Any]) -> None:
    assert summary["memory_cell_id"], summary
    assert summary["ttl_seconds"] == 3600, summary
    assert summary["context_cells"] >= 2, summary
    assert summary["verify_verdict"] == "mixed_evidence", summary
    assert "remembered preference" in summary["answer"], summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    parser.add_argument("--scope", default=DEFAULT_SCOPE)
    parser.add_argument("--question", default=DEFAULT_QUESTION)
    parser.add_argument("--preference", default=DEFAULT_PREFERENCE)
    parser.add_argument("--fact", default=DEFAULT_FACT)
    parser.add_argument("--no-reset", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = run_demo(
        args.db,
        args.scope,
        args.question,
        args.preference,
        args.fact,
        reset=not args.no_reset,
    )
    if args.self_test:
        assert_self_test(summary)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
