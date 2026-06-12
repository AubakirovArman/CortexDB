#!/usr/bin/env python3
"""Tests for anchor/evidence-overlap diagnostics."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from anchor_overlap_diagnostics import overlap_count, query_anchors


class AnchorOverlapDiagnosticsTests(unittest.TestCase):
    def test_query_anchors_extract_paths_tickets_and_meaningful_terms(self) -> None:
        anchors = query_anchors("What changed in AUTH-123 for /api/files upload limits?")
        self.assertIn("AUTH-123", anchors)
        self.assertIn("/api/files", anchors)
        self.assertIn("upload", anchors)
        self.assertIn("limits", anchors)
        self.assertNotIn("what", anchors)

    def test_overlap_count_matches_exact_anchors_case_insensitively(self) -> None:
        anchors = ["AUTH-123", "/api/files", "upload"]
        text = "The auth-123 runbook documents upload behavior."
        self.assertEqual(2, overlap_count(anchors, text))


if __name__ == "__main__":
    unittest.main()
