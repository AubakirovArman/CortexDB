#!/usr/bin/env python3
"""Tests for answer guard helpers."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from answer_guard import concrete_markers, guard_unsupported_claims


class AnswerGuardTests(unittest.TestCase):
    def test_concrete_markers_find_dates_numbers_ids_and_paths(self) -> None:
        markers = concrete_markers(
            "Use JIRA-123 by June 12, 2026 with 30 days TTL at /run/books/x."
        )
        self.assertIn("JIRA-123", markers)
        self.assertIn("June 12, 2026", markers)
        self.assertIn("2026", markers)
        self.assertIn("30 days", markers)
        self.assertIn("/run/books/x", markers)

    def test_report_mode_does_not_change_answer(self) -> None:
        answer = "The timeout is 45 seconds."
        guarded, report = guard_unsupported_claims(
            answer,
            "The runbook mentions timeout but not a value.",
            mode="report",
        )
        self.assertEqual(answer, guarded)
        self.assertEqual(0, report["removed_count"])
        self.assertEqual(1, report["flagged_count"])
        self.assertIn("45 seconds", report["unsupported_markers"])

    def test_suppress_mode_removes_unsupported_sentence(self) -> None:
        guarded, report = guard_unsupported_claims(
            "Use the new endpoint. The timeout is 45 seconds.",
            "Use the new endpoint after the rollout.",
            mode="suppress",
        )
        self.assertEqual("Use the new endpoint.", guarded)
        self.assertEqual(1, report["removed_count"])

    def test_suppress_mode_keeps_supported_marker(self) -> None:
        guarded, report = guard_unsupported_claims(
            "The timeout is 45 seconds.",
            "The timeout is 45 seconds in the incident runbook.",
            mode="suppress",
        )
        self.assertEqual("The timeout is 45 seconds.", guarded)
        self.assertEqual(0, report["removed_count"])

    def test_suppress_mode_returns_insufficient_when_everything_removed(self) -> None:
        guarded, report = guard_unsupported_claims(
            "The TTL is 7 days.",
            "No TTL is stated.",
            mode="suppress",
        )
        self.assertEqual("Insufficient information.", guarded)
        self.assertEqual(1, report["removed_count"])

    def test_repair_mode_rewrites_subject_value_without_removing_sentence(self) -> None:
        guarded, report = guard_unsupported_claims(
            "Use the new endpoint. The timeout is 45 seconds.",
            "Use the new endpoint after the rollout.",
            mode="repair",
        )
        self.assertEqual(
            "Use the new endpoint. The timeout is not stated in the retrieved evidence.",
            guarded,
        )
        self.assertEqual(0, report["removed_count"])
        self.assertEqual(1, report["repaired_count"])
        self.assertEqual(1, report["flagged_count"])
        self.assertIn("45 seconds", report["unsupported_markers"])

    def test_repair_mode_preserves_supported_markers_and_removes_only_missing_values(self) -> None:
        guarded, report = guard_unsupported_claims(
            "Use /v1/files with timeout 45 seconds.",
            "The supported path is /v1/files.",
            mode="repair",
        )
        self.assertIn("/v1/files", guarded)
        self.assertNotIn("45 seconds", guarded)
        self.assertEqual(1, report["repaired_count"])


if __name__ == "__main__":
    unittest.main()
