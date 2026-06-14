#!/usr/bin/env python3
"""Tests for oracle-free answer intent and budget profiles."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent))

from answer_intent import answer_intent_profile
from deepseek_answers_lib.budget import answer_budget_trace_row, resolve_answer_budget


class AnswerIntentTests(unittest.TestCase):
    def test_high_level_profile_selects_brain_digest_budget(self) -> None:
        profile = answer_intent_profile(
            "What are the company mission, product strategy, and revenue streams?"
        )
        self.assertEqual("high_level", profile["intent"])
        self.assertEqual("brain-digest", profile["budget_profile"]["context_mode"])
        self.assertEqual(10, profile["budget_profile"]["top_k_context"])

    def test_completeness_profile_gets_larger_budget(self) -> None:
        profile = answer_intent_profile(
            "List all required rollout checks, owners, dashboards, and mitigation steps."
        )
        self.assertEqual("completeness", profile["intent"])
        self.assertGreaterEqual(profile["budget_profile"]["max_tokens"], 900)

    def test_default_profile_does_not_force_budget(self) -> None:
        profile = answer_intent_profile("What is the upload limit?")
        self.assertEqual("default", profile["intent"])
        self.assertIsNone(profile["budget_profile"]["max_tokens"])

    def test_resolve_answer_budget_exposes_active_budget_without_oracle_type(self) -> None:
        args = SimpleNamespace(
            context_mode="question-window-digest-ranked",
            top_k_context=8,
            max_chars_per_doc=2200,
            max_tokens=420,
            enable_text_intent_budget=True,
            complex_top_k_context=10,
            complex_max_chars_per_doc=2600,
            complex_max_tokens=900,
            high_level_context_mode="brain-digest",
            high_level_top_k_context=10,
            high_level_max_chars_per_doc=2600,
            high_level_max_tokens=900,
        )

        budget = resolve_answer_budget(
            question="List all required rollout checks, owners, dashboards, and mitigation steps.",
            args=args,
        )

        self.assertEqual("completeness", budget["answer_intent"])
        self.assertEqual(10, budget["top_k_context"])
        self.assertGreaterEqual(budget["max_tokens"], 900)
        self.assertTrue(budget["adaptive_budget_applied"])
        self.assertFalse(budget["high_level_override_applied"])

    def test_resolve_answer_budget_keeps_simple_lookup_compact(self) -> None:
        args = SimpleNamespace(
            context_mode="question-window-digest-ranked",
            top_k_context=8,
            max_chars_per_doc=2200,
            max_tokens=420,
            enable_text_intent_budget=True,
            complex_top_k_context=10,
            complex_max_chars_per_doc=2600,
            complex_max_tokens=900,
            high_level_context_mode="brain-digest",
            high_level_top_k_context=10,
            high_level_max_chars_per_doc=2600,
            high_level_max_tokens=900,
        )

        budget = resolve_answer_budget(
            question="What is the upload limit?",
            args=args,
        )

        self.assertEqual("default", budget["answer_intent"])
        self.assertEqual(8, budget["top_k_context"])
        self.assertEqual(420, budget["max_tokens"])
        self.assertFalse(budget["adaptive_budget_applied"])

    def test_answer_budget_trace_row_recomputes_without_llm_answer(self) -> None:
        args = SimpleNamespace(
            context_mode="question-window-digest-ranked",
            top_k_context=8,
            max_chars_per_doc=2200,
            max_tokens=420,
            enable_text_intent_budget=True,
            complex_top_k_context=10,
            complex_max_chars_per_doc=2600,
            complex_max_tokens=900,
            high_level_context_mode="brain-digest",
            high_level_top_k_context=10,
            high_level_max_chars_per_doc=2600,
            high_level_max_tokens=900,
        )

        trace = answer_budget_trace_row(
            {
                "question_id": "q1",
                "question": "List all required rollout checks, owners, dashboards, and mitigation steps.",
                "document_ids": ["d1", "d2", "d3"],
            },
            None,
            args,
        )

        self.assertEqual("q1", trace["question_id"])
        self.assertEqual(10, trace["selected_result_limit"])
        self.assertEqual(3, trace["used_doc_count"])
        self.assertEqual("recomputed", trace["trace_source"])
        self.assertTrue(trace["adaptive_budget_applied"])


if __name__ == "__main__":
    unittest.main()
