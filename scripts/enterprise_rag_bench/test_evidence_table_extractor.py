#!/usr/bin/env python3
"""Tests for deterministic evidence-table extraction."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from evidence_table_extractor import extract_evidence_table, format_evidence_table_for_prompt


class EvidenceTableExtractorTests(unittest.TestCase):
    def test_markdown_table_rows_emit_structured_cells(self) -> None:
        content = """
| Project | Owner | Deadline | Status |
| --- | --- | --- | --- |
| Apollo | Maya | 2026-06-12 | blocked |
| Boreal | Ken | 2026-07-01 | shipped |
"""
        facts = extract_evidence_table(
            doc_id="doc-1",
            title="Project status table",
            content=content,
            question="Who owns Apollo and what is its status?",
            max_facts=10,
        )
        structured = [
            fact
            for fact in facts
            if "structured_table_row" in fact.get("fact_types", [])
            and "Apollo" in str(fact.get("text"))
        ]
        self.assertTrue(structured)
        cells = structured[0]["table_cells"]
        self.assertIn({"header": "Project", "value": "Apollo"}, cells)
        self.assertIn({"header": "Owner", "value": "Maya"}, cells)
        self.assertIn({"header": "Status", "value": "blocked"}, cells)

    def test_prompt_formatter_includes_structured_key_values(self) -> None:
        table = {
            "question_id": "q1",
            "facts": [
                {
                    "doc_id": "doc-1",
                    "title": "Project status table",
                    "line": 4,
                    "fact_types": ["structured_table_row", "table_row"],
                    "score": 10,
                    "text": "Project: Apollo | Owner: Maya",
                    "table_cells": [
                        {"header": "Project", "value": "Apollo"},
                        {"header": "Owner", "value": "Maya"},
                    ],
                }
            ],
        }
        prompt = format_evidence_table_for_prompt(table)
        self.assertIn("Project=Apollo", prompt)
        self.assertIn("Owner=Maya", prompt)
        self.assertIn("structured_table_row", prompt)


if __name__ == "__main__":
    unittest.main()
