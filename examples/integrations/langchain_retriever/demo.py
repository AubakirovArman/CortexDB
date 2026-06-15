#!/usr/bin/env python3
"""Dependency-free LangChain-style retriever example for CortexDB."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "common"))
from cortex_cli import reset_fixture_database, retrieve_context


DEFAULT_DB = Path("target/live-integration-examples/langchain-retriever/db")
DEFAULT_SCOPE = "project:investments"
DEFAULT_QUERY = "Solar Plant budget"


@dataclass(frozen=True)
class Document:
    page_content: str
    metadata: dict[str, Any]


class CortexRetriever:
    def __init__(self, db_path: Path, scope: str, limit: int = 6) -> None:
        self.db_path = db_path
        self.scope = scope
        self.limit = limit

    def invoke(self, query: str) -> list[Document]:
        pack = retrieve_context(self.db_path, self.scope, query, self.limit)
        docs: list[Document] = []
        for cell in pack.get("cells", []):
            if not isinstance(cell, dict):
                continue
            docs.append(
                Document(
                    page_content=str(cell.get("payload_text", "")),
                    metadata={
                        "cell_id": cell.get("cell_id"),
                        "citation": cell.get("citation"),
                        "source_id": (cell.get("source_ref") or {}).get("source_id"),
                        "answerability_q16": pack.get("answerability_q16"),
                        "visible_conflict_count": pack.get("visible_conflict_count"),
                    },
                )
            )
        return docs


class MockRagAnswerer:
    def answer(self, question: str, documents: list[Document]) -> str:
        citations = [doc.metadata.get("citation") for doc in documents[:2]]
        return (
            f"Question: {question}. Retrieved {len(documents)} CortexDB documents "
            f"with citations {citations}."
        )


def run_demo(db_path: Path, scope: str, query: str, reset: bool) -> dict[str, Any]:
    if reset:
        reset_fixture_database(db_path)
    retriever = CortexRetriever(db_path, scope)
    documents = retriever.invoke(query)
    answer = MockRagAnswerer().answer(query, documents)
    return {
        "schema_version": "cortexdb.integration.langchain_retriever.v1",
        "query": query,
        "document_count": len(documents),
        "documents": [asdict(doc) for doc in documents[:3]],
        "answer": answer,
    }


def assert_self_test(summary: dict[str, Any]) -> None:
    assert summary["document_count"] >= 2, summary
    citations = [doc["metadata"]["citation"] for doc in summary["documents"]]
    assert "report_q1.pdf#page=3" in citations or "report_q2.pdf#page=5" in citations, summary
    assert "Retrieved" in summary["answer"], summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    parser.add_argument("--scope", default=DEFAULT_SCOPE)
    parser.add_argument("--query", default=DEFAULT_QUERY)
    parser.add_argument("--no-reset", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = run_demo(args.db, args.scope, args.query, reset=not args.no_reset)
    if args.self_test:
        assert_self_test(summary)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
