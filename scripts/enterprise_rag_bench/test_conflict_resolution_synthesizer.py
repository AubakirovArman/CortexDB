#!/usr/bin/env python3
"""Tests for conflict-resolution evidence plans."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from conflict_resolution_synthesizer import build_conflict_plan
from evidence_slot_planner import format_evidence_plan_for_prompt


class ConflictResolutionSynthesizerTests(unittest.TestCase):
    def test_builds_current_and_previous_claims(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sources = root / "sources"
            sources.mkdir()
            doc = {
                "title_field_name": "title",
                "content_field_names": ["body"],
                "title": "Policy update",
                "body": "\n".join(
                    [
                        "Previous policy used timeout 30 seconds before 2026-01-01.",
                        "Current policy as of 2026-06-01 requires timeout 45 seconds.",
                    ]
                ),
            }
            (sources / "doc.json").write_text(json.dumps(doc), encoding="utf-8")
            plan = build_conflict_plan(
                {
                    "question_id": "q1",
                    "question": "What is the current timeout and what was previous?",
                    "document_ids": ["doc-1"],
                },
                {"doc-1": "doc.json"},
                sources,
                top_docs=1,
                max_rows_total=10,
            )
        by_kind = plan["conflict_resolution"]["by_kind"]
        self.assertIn("current", by_kind)
        self.assertIn("previous", by_kind)
        prompt = format_evidence_plan_for_prompt(plan)
        self.assertIn("Conflict-resolution claims", prompt)
        self.assertIn("45 seconds", prompt)
        self.assertIn("30 seconds", prompt)


if __name__ == "__main__":
    unittest.main()
