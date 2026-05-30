#!/usr/bin/env python3
"""Compare two ann_corpus_check JSON reports."""

from __future__ import annotations

import argparse
import json
import sys
import unittest
from pathlib import Path
from typing import Any


def load_report(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error


def compare_reports(
    baseline: dict[str, Any],
    candidate: dict[str, Any],
    max_p95_regression_nanos: int,
    max_max_regression_nanos: int,
) -> tuple[bool, list[str], dict[str, Any]]:
    failures: list[str] = []
    deltas = {
        "min_recall_delta_q16": int(candidate["min_observed_recall_q16"])
        - int(baseline["min_observed_recall_q16"]),
        "mean_recall_delta_q16": int(candidate["mean_recall_q16"])
        - int(baseline["mean_recall_q16"]),
        "p95_latency_delta_nanos": int(candidate["p95_latency_nanos"])
        - int(baseline["p95_latency_nanos"]),
        "max_latency_delta_nanos": int(candidate["max_latency_nanos"])
        - int(baseline["max_latency_nanos"]),
    }
    if baseline.get("metric") != candidate.get("metric"):
        failures.append(f"metric changed: {baseline.get('metric')} -> {candidate.get('metric')}")
    if candidate.get("vector_count") != baseline.get("vector_count"):
        failures.append("vector_count changed")
    if candidate.get("query_count") != baseline.get("query_count"):
        failures.append("query_count changed")
    if candidate.get("dimension") != baseline.get("dimension"):
        failures.append("dimension changed")
    if deltas["min_recall_delta_q16"] < 0:
        failures.append(f"min recall regressed by {abs(deltas['min_recall_delta_q16'])}")
    if deltas["mean_recall_delta_q16"] < 0:
        failures.append(f"mean recall regressed by {abs(deltas['mean_recall_delta_q16'])}")
    if deltas["p95_latency_delta_nanos"] > max_p95_regression_nanos:
        failures.append("p95 latency regression exceeded budget")
    if deltas["max_latency_delta_nanos"] > max_max_regression_nanos:
        failures.append("max latency regression exceeded budget")
    if baseline.get("production_safe") and not candidate.get("production_safe"):
        failures.append("production_safe changed from true to false")
    return not failures, failures, deltas


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--max-p95-regression-nanos", type=int, default=0)
    parser.add_argument("--max-max-regression-nanos", type=int, default=0)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    passed, failures, deltas = compare_reports(
        load_report(args.baseline),
        load_report(args.candidate),
        args.max_p95_regression_nanos,
        args.max_max_regression_nanos,
    )
    result = {"passed": passed, "failures": failures, "deltas": deltas}
    text = json.dumps(result, separators=(",", ":")) + "\n"
    if args.output is None:
        sys.stdout.write(text)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
    return 0 if passed else 1


class SelfTests(unittest.TestCase):
    def report(self, recall: int, p95: int, safe: bool = True) -> dict[str, Any]:
        return {
            "metric": "dot_product",
            "vector_count": 2,
            "query_count": 1,
            "dimension": 2,
            "min_observed_recall_q16": recall,
            "mean_recall_q16": recall,
            "p95_latency_nanos": p95,
            "max_latency_nanos": p95,
            "production_safe": safe,
        }

    def test_recall_regression_fails(self) -> None:
        passed, failures, _ = compare_reports(self.report(65535, 10), self.report(49151, 10), 0, 0)
        self.assertFalse(passed)
        self.assertTrue(any("recall" in failure for failure in failures))

    def test_latency_budget_allows_small_delta(self) -> None:
        passed, failures, deltas = compare_reports(
            self.report(65535, 10),
            self.report(65535, 15),
            5,
            5,
        )
        self.assertTrue(passed, failures)
        self.assertEqual(deltas["p95_latency_delta_nanos"], 5)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
