#!/usr/bin/env python3
"""OpenAI/Anthropic style tool-calling example backed by CortexDB."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "common"))
from cortex_cli import citations, reset_fixture_database, retrieve_context, verify_fact


DEFAULT_DB = Path("target/live-integration-examples/llm-tool-calling/db")
DEFAULT_SCOPE = "project:investments"
DEFAULT_QUESTION = "What is the Solar Plant budget?"
DEFAULT_FACT = "Solar Plant budget is 1.2B KZT"


def openai_tools() -> list[dict[str, Any]]:
    return [
        {
            "type": "function",
            "function": {
                "name": "retrieve_context",
                "description": "Retrieve a citation-aware CortexDB ContextPack.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1, "maximum": 20},
                    },
                    "required": ["task"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "verify_fact",
                "description": "Verify a factual claim against scoped CortexDB evidence.",
                "parameters": {
                    "type": "object",
                    "properties": {"fact": {"type": "string"}},
                    "required": ["fact"],
                },
            },
        },
    ]


def anthropic_tools() -> list[dict[str, Any]]:
    return [
        {
            "name": "retrieve_context",
            "description": "Retrieve a citation-aware CortexDB ContextPack.",
            "input_schema": openai_tools()[0]["function"]["parameters"],
        },
        {
            "name": "verify_fact",
            "description": "Verify a factual claim against scoped CortexDB evidence.",
            "input_schema": openai_tools()[1]["function"]["parameters"],
        },
    ]


@dataclass(frozen=True)
class ToolCall:
    name: str
    arguments: dict[str, Any]


class MockToolCallingModel:
    def plan(self, question: str, fact: str) -> list[ToolCall]:
        return [
            ToolCall("retrieve_context", {"task": question, "limit": 10}),
            ToolCall("verify_fact", {"fact": fact}),
        ]

    def final_answer(self, context_pack: dict[str, Any], verification: dict[str, Any]) -> str:
        refs = ", ".join(citations(context_pack)[:2])
        verdict = verification.get("verdict")
        return (
            f"CortexDB returned cited evidence ({refs}) and VERIFY reported "
            f"{verdict}; the answer should mention the visible budget conflict."
        )


class CortexToolHandler:
    def __init__(self, db_path: Path, scope: str) -> None:
        self.db_path = db_path
        self.scope = scope

    def handle(self, call: ToolCall) -> dict[str, Any]:
        if call.name == "retrieve_context":
            return retrieve_context(
                self.db_path,
                self.scope,
                call.arguments["task"],
                int(call.arguments.get("limit", 10)),
            )
        if call.name == "verify_fact":
            return verify_fact(self.db_path, self.scope, call.arguments["fact"])
        raise ValueError(f"unknown tool call: {call.name}")


def run_demo(db_path: Path, scope: str, question: str, fact: str, reset: bool) -> dict[str, Any]:
    if reset:
        reset_fixture_database(db_path)

    model = MockToolCallingModel()
    handler = CortexToolHandler(db_path, scope)
    outputs = {call.name: handler.handle(call) for call in model.plan(question, fact)}
    answer = model.final_answer(outputs["retrieve_context"], outputs["verify_fact"])
    summary = {
        "schema_version": "cortexdb.integration.llm_tool_calling.v1",
        "openai_tool_names": [tool["function"]["name"] for tool in openai_tools()],
        "anthropic_tool_names": [tool["name"] for tool in anthropic_tools()],
        "context_cells": len(outputs["retrieve_context"].get("cells", [])),
        "citations": citations(outputs["retrieve_context"])[:3],
        "verify_verdict": outputs["verify_fact"].get("verdict"),
        "answer": answer,
    }
    return summary


def assert_self_test(summary: dict[str, Any]) -> None:
    assert summary["openai_tool_names"] == ["retrieve_context", "verify_fact"], summary
    assert summary["anthropic_tool_names"] == ["retrieve_context", "verify_fact"], summary
    assert summary["context_cells"] >= 2, summary
    assert "report_q1.pdf#page=3" in summary["citations"], summary
    assert summary["verify_verdict"] == "mixed_evidence", summary
    assert "visible budget conflict" in summary["answer"], summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    parser.add_argument("--scope", default=DEFAULT_SCOPE)
    parser.add_argument("--question", default=DEFAULT_QUESTION)
    parser.add_argument("--fact", default=DEFAULT_FACT)
    parser.add_argument("--no-reset", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = run_demo(args.db, args.scope, args.question, args.fact, reset=not args.no_reset)
    if args.self_test:
        assert_self_test(summary)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
