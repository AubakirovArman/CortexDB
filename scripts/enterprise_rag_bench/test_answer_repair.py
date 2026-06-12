#!/usr/bin/env python3
"""Tests for self-consistency answer repair helpers."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from answer_repair import (
    build_self_consistency_repair_prompt,
    should_self_consistency_repair,
)


class AnswerRepairTests(unittest.TestCase):
    def test_should_self_consistency_repair_uses_flagged_count(self) -> None:
        self.assertFalse(should_self_consistency_repair({"flagged_count": 0}))
        self.assertFalse(should_self_consistency_repair({}))
        self.assertTrue(should_self_consistency_repair({"flagged_count": 2}))

    def test_repair_prompt_contains_evidence_and_guard_markers(self) -> None:
        prompt = build_self_consistency_repair_prompt(
            question="What timeout does the runbook state?",
            context="Retrieved evidence says the runbook names the endpoint only.",
            draft_answer="The timeout is 45 seconds.",
            guard_report={"unsupported_markers": ["45 seconds"]},
        )
        self.assertIn("What timeout does the runbook state?", prompt)
        self.assertIn("45 seconds", prompt)
        self.assertIn("The timeout is 45 seconds.", prompt)
        self.assertIn("Retrieved evidence says the runbook", prompt)
        self.assertIn("Repaired final answer:", prompt)
        self.assertIn("Do not invent replacement values.", prompt)


if __name__ == "__main__":
    unittest.main()
