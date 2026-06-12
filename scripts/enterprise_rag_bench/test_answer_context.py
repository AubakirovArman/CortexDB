#!/usr/bin/env python3
"""Tests for EnterpriseRAG answer context builders."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from answer_context import brain_digest_context, brain_digest_score


class AnswerContextTests(unittest.TestCase):
    def test_brain_digest_selects_overview_themes(self) -> None:
        content = """
Random implementation detail about a small retry loop.
Mission: CortexDB helps agent workflows retrieve grounded context from durable memory.
Platform strategy: product teams use the API, audit trail, and retrieval runtime together.
Security policy: tenant RBAC and audit logging are required for enterprise deployments.
Reliability target: p95 retrieval latency must stay inside the documented SLO.
"""
        digest = brain_digest_context(
            content,
            "Company overview",
            "What is the company mission, platform strategy, security posture, and reliability approach?",
            1200,
        )
        self.assertIn("Mode: brain_digest", digest)
        self.assertIn("mission_strategy", digest)
        self.assertIn("product_platform", digest)
        self.assertIn("security_compliance", digest)
        self.assertIn("reliability_operations", digest)

    def test_brain_digest_score_rewards_theme_matches(self) -> None:
        content = "Security policy requires tenant RBAC. Pricing plan supports add-ons."
        score = brain_digest_score(content, "What are the security and pricing policies?")
        self.assertGreater(score, 0.0)


if __name__ == "__main__":
    unittest.main()
