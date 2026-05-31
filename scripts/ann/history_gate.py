#!/usr/bin/env python3
"""Gate archived ANN report history for release evidence."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

from summarize_history import summarize_history, write_summary


def validate_history(
    summary: dict[str, Any],
    *,
    min_runs: int,
    min_corpora: int,
    fail_on_regression: bool,
) -> list[str]:
    errors: list[str] = []
    if summary["run_count"] < min_runs:
        errors.append(f"expected at least {min_runs} ANN run(s), found {summary['run_count']}")
    if summary["corpus_count"] < min_corpora:
        errors.append(f"expected at least {min_corpora} ANN corpus group(s), found {summary['corpus_count']}")
    if fail_on_regression and summary["regression_count"] > 0:
        errors.append(f"found {summary['regression_count']} ANN history regression(s)")
    return errors


def run_gate(args: argparse.Namespace) -> int:
    summary = summarize_history(args.run_root)
    write_summary(summary, args.output)
    errors = validate_history(
        summary,
        min_runs=args.min_runs,
        min_corpora=args.min_corpora,
        fail_on_regression=args.fail_on_regression,
    )
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-root", type=Path, default=Path("target/ann/corpus-runs"))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--fail-on-regression", action="store_true")
    parser.add_argument("--min-runs", type=int, default=1)
    parser.add_argument("--min-corpora", type=int, default=1)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    return run_gate(parse_args(argv))


class SelfTests(unittest.TestCase):
    def write_run(self, root: Path, run_id: str, recall: int, p95: int) -> None:
        run_dir = root / run_id
        run_dir.mkdir(parents=True)
        manifest = {
            "run_id": run_id,
            "git_sha": run_id,
            "metric": "cosine",
            "vectors": "vectors.jsonl",
            "queries": "queries.jsonl",
            "ground_truth": "ground_truth.jsonl",
            "baseline_report": "",
            "report": str(run_dir / "report.json"),
        }
        report = {
            "passed": True,
            "production_safe": True,
            "metric": "cosine",
            "vector_count": 2,
            "query_count": 1,
            "dimension": 2,
            "graph_nodes": 2,
            "graph_edges": 2,
            "upper_layers": 1,
            "upper_graph_edges": 1,
            "min_observed_recall_q16": recall,
            "mean_recall_q16": recall,
            "p50_latency_nanos": p95,
            "p95_latency_nanos": p95,
            "max_latency_nanos": p95,
        }
        (run_dir / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (run_dir / "report.json").write_text(json.dumps(report), encoding="utf-8")

    def test_empty_history_fails_minimum_gate(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            summary = summarize_history(Path(raw_dir))
        errors = validate_history(summary, min_runs=1, min_corpora=1, fail_on_regression=True)
        self.assertEqual(len(errors), 2)

    def test_gate_accepts_single_safe_run(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            self.write_run(Path(raw_dir), "run-a", 65535, 10)
            summary = summarize_history(Path(raw_dir))
        errors = validate_history(summary, min_runs=1, min_corpora=1, fail_on_regression=True)
        self.assertEqual(errors, [])

    def test_gate_fails_regression(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            self.write_run(root, "run-a", 65535, 10)
            self.write_run(root, "run-b", 60000, 20)
            summary = summarize_history(root)
        errors = validate_history(summary, min_runs=2, min_corpora=1, fail_on_regression=True)
        self.assertTrue(any("regression" in error for error in errors))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
