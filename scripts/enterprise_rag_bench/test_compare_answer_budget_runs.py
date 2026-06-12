#!/usr/bin/env python3
"""Tests for answer budget A/B reports."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent))

from compare_answer_budget_runs import build_report, extract_judge_metrics


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.write_text(
        "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


class CompareAnswerBudgetRunsTests(unittest.TestCase):
    def test_build_report_compares_candidate_to_static_budget(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            trace = root / "candidate.jsonl"
            report = root / "report.json"
            write_jsonl(
                trace,
                [
                    {
                        "question_id": "q1",
                        "answer_intent": "default",
                        "selected_result_limit": 8,
                        "active_max_chars_per_doc": 2200,
                        "active_max_tokens": 420,
                        "retrieved_doc_count": 10,
                        "used_doc_count": 8,
                        "adaptive_budget_applied": False,
                    },
                    {
                        "question_id": "q2",
                        "answer_intent": "complex_project",
                        "selected_result_limit": 10,
                        "active_max_chars_per_doc": 2600,
                        "active_max_tokens": 900,
                        "retrieved_doc_count": 10,
                        "used_doc_count": 10,
                        "adaptive_budget_applied": True,
                    },
                ],
            )

            payload = build_report(
                SimpleNamespace(
                    candidate_trace=trace,
                    baseline_trace=None,
                    candidate_answer_report=None,
                    baseline_answer_report=None,
                    candidate_judge_results=None,
                    baseline_judge_results=None,
                    report=report,
                    markdown=None,
                    static_top_k_context=8,
                    static_max_chars_per_doc=2200,
                    static_max_tokens=420,
                )
            )

            self.assertEqual("derived_static", payload["baseline_mode"])
            self.assertEqual(2, payload["candidate_trace"]["questions"])
            self.assertEqual(1, payload["candidate_trace"]["adaptive_budget_questions"])
            self.assertEqual(1.0, payload["trace_delta"]["avg_used_doc_count"])
            self.assertEqual(240.0, payload["trace_delta"]["avg_max_tokens"])

    def test_extract_judge_metrics_supports_official_results_shape(self) -> None:
        payload = extract_judge_metrics(
            {
                "aggregate_stats": {
                    "combined_correctness_completeness_score": 43.27,
                    "average_correctness_pct": 69.2,
                    "average_completeness_pct": 71.2,
                },
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
            }
        )

        self.assertEqual(43.27, payload["overall"])
        self.assertEqual(69.2, payload["answer_correctness_pct"])
        self.assertEqual(71.2, payload["answer_completeness_pct"])
        self.assertEqual(15, payload["total_tokens"])


if __name__ == "__main__":
    unittest.main()
