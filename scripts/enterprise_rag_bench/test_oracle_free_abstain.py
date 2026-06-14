#!/usr/bin/env python3
"""Tests for oracle-free abstain classifier."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from oracle_free_abstain import abstain_decision, is_high_level_question, requires_exact_literal


class OracleFreeAbstainTests(unittest.TestCase):
    def test_high_level_question_detected(self) -> None:
        self.assertTrue(
            is_high_level_question("What is Redwood Inference's mission and business strategy?")
        )
        self.assertTrue(
            is_high_level_question("Describe the company's security posture and key departments.")
        )

    def test_project_question_not_high_level(self) -> None:
        self.assertFalse(
            is_high_level_question("What caused the incident PROJ-123 and how was it mitigated?")
        )

    def test_literal_question_requires_value(self) -> None:
        self.assertTrue(requires_exact_literal("What is the exact value of the upload limit?"))
        self.assertTrue(requires_exact_literal("How many seats were provisioned for tenant ACME-42?"))

    def test_high_level_does_not_require_literal(self) -> None:
        self.assertFalse(
            requires_exact_literal("What is Redwood Inference's mission and business strategy?")
        )

    def test_abstain_with_documents(self) -> None:
        should_abstain, reason = abstain_decision(
            question="What is the value of X?", document_ids=["doc_1"]
        )
        self.assertFalse(should_abstain)
        self.assertEqual(reason, "has_retrieved_documents")

    def test_high_level_zero_docs_does_not_abstain(self) -> None:
        should_abstain, reason = abstain_decision(
            question="What is Redwood Inference's mission and business strategy?",
            document_ids=[],
        )
        self.assertFalse(should_abstain)
        self.assertEqual(reason, "high_level_company_scope")

    def test_literal_zero_docs_abstains(self) -> None:
        should_abstain, reason = abstain_decision(
            question="What is the exact upload limit in MiB?", document_ids=[]
        )
        self.assertTrue(should_abstain)
        self.assertEqual(reason, "no_evidence_for_literal")

    def test_generic_zero_docs_abstains(self) -> None:
        should_abstain, reason = abstain_decision(
            question="Tell me about the latest rollout.", document_ids=[]
        )
        self.assertTrue(should_abstain)
        self.assertEqual(reason, "no_evidence")


if __name__ == "__main__":
    unittest.main()
