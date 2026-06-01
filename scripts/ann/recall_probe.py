#!/usr/bin/env python3
"""Run repeated ANN corpus probes and summarize recall/latency stability."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "cortexdb.ann_recall_probe.v1"

def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def int_value(report: dict[str, Any], field: str) -> int:
    value = report.get(field)
    if not isinstance(value, int):
        raise RuntimeError(f"{field}: expected integer")
    return value


def bool_value(report: dict[str, Any], field: str) -> bool:
    value = report.get(field)
    if not isinstance(value, bool):
        raise RuntimeError(f"{field}: expected boolean")
    return value


def run_probe(args: argparse.Namespace) -> dict[str, Any]:
    reports: list[dict[str, Any]] = []
    failures: list[str] = []
    with tempfile.TemporaryDirectory() as raw_tmp:
        tmp = Path(raw_tmp)
        for index in range(args.iterations):
            output = tmp / f"probe-{index + 1}.json"
            command = [
                str(args.runner),
                "--vectors",
                str(args.vectors),
                "--queries",
                str(args.queries),
                "--ground-truth",
                str(args.ground_truth),
                "--metric",
                args.metric,
                "--min-recall-q16",
                str(args.min_recall_q16),
                "--min-mean-recall-q16",
                str(args.min_mean_recall_q16),
                "--max-neighbors",
                str(args.max_neighbors),
                "--ef-search",
                str(args.ef_search),
                "--ef-construction",
                str(args.ef_construction),
                "--layer-count",
                str(args.layer_count),
                "--max-p95-latency-nanos",
                str(args.max_p95_latency_nanos),
                "--max-p99-latency-nanos",
                str(args.max_p99_latency_nanos),
                "--max-max-latency-nanos",
                str(args.max_max_latency_nanos),
                "--output",
                str(output),
            ]
            completed = subprocess.run(
                command,
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            if completed.returncode != 0:
                failures.append(
                    f"iteration {index + 1}: runner failed: {completed.stderr.strip()}"
                )
                continue
            reports.append(load_json(output))
    return summarize_reports(
        reports,
        failures,
        args.iterations,
        min_recall_q16=args.min_recall_q16,
        min_mean_recall_q16=args.min_mean_recall_q16,
    )


def summarize_reports(
    reports: list[dict[str, Any]],
    failures: list[str],
    expected_iterations: int,
    *,
    min_recall_q16: int = 49_151,
    min_mean_recall_q16: int = 49_151,
) -> dict[str, Any]:
    if len(reports) != expected_iterations:
        failures.append(
            f"expected {expected_iterations} probe report(s), found {len(reports)}"
        )
    for index, report in enumerate(reports, start=1):
        validate_report(
            report,
            index,
            failures,
            min_recall_q16=min_recall_q16,
            min_mean_recall_q16=min_mean_recall_q16,
        )
    if reports:
        first = graph_shape(reports[0])
        for index, report in enumerate(reports[1:], start=2):
            if graph_shape(report) != first:
                failures.append(f"iteration {index}: graph shape changed")
    summary = {
        "schema_version": SCHEMA_VERSION,
        "passed": not failures,
        "production_safe": bool(reports) and not failures,
        "iterations": len(reports),
        "expected_iterations": expected_iterations,
        "failures": failures,
        "min_observed_recall_q16_min": min_int(reports, "min_observed_recall_q16"),
        "mean_recall_q16_min": min_int(reports, "mean_recall_q16"),
        "p95_latency_nanos_max": max_int(reports, "p95_latency_nanos"),
        "p99_latency_nanos_max": max_int(reports, "p99_latency_nanos"),
        "max_latency_nanos_max": max_int(reports, "max_latency_nanos"),
        "graph_shape": graph_shape(reports[0]) if reports else {},
        "reports": compact_reports(reports),
    }
    return summary


def validate_report(
    report: dict[str, Any],
    index: int,
    failures: list[str],
    *,
    min_recall_q16: int,
    min_mean_recall_q16: int,
) -> None:
    prefix = f"iteration {index}"
    if bool_value(report, "passed") is not True:
        failures.append(f"{prefix}: passed must be true")
    if bool_value(report, "production_safe") is not True:
        failures.append(f"{prefix}: production_safe must be true")
    if int_value(report, "min_observed_recall_q16") < min_recall_q16:
        failures.append(f"{prefix}: min_observed_recall_q16 below probe threshold")
    if int_value(report, "mean_recall_q16") < min_mean_recall_q16:
        failures.append(f"{prefix}: mean_recall_q16 below probe threshold")
    if int_value(report, "p99_latency_nanos") <= 0:
        failures.append(f"{prefix}: p99_latency_nanos must be positive")
    if int_value(report, "upper_layers") <= 0:
        failures.append(f"{prefix}: upper_layers must be positive")
    if int_value(report, "upper_graph_edges") <= 0:
        failures.append(f"{prefix}: upper_graph_edges must be positive")


def graph_shape(report: dict[str, Any]) -> dict[str, int]:
    return {
        "vector_count": int_value(report, "vector_count"),
        "query_count": int_value(report, "query_count"),
        "dimension": int_value(report, "dimension"),
        "graph_nodes": int_value(report, "graph_nodes"),
        "graph_edges": int_value(report, "graph_edges"),
        "upper_layers": int_value(report, "upper_layers"),
        "upper_graph_edges": int_value(report, "upper_graph_edges"),
    }


def min_int(reports: list[dict[str, Any]], field: str) -> int:
    return min((int_value(report, field) for report in reports), default=0)


def max_int(reports: list[dict[str, Any]], field: str) -> int:
    return max((int_value(report, field) for report in reports), default=0)


def compact_reports(reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    fields = [
        "passed",
        "production_safe",
        "min_observed_recall_q16",
        "mean_recall_q16",
        "p95_latency_nanos",
        "p99_latency_nanos",
        "max_latency_nanos",
        "graph_nodes",
        "graph_edges",
        "upper_layers",
        "upper_graph_edges",
    ]
    return [{field: report.get(field) for field in fields} for report in reports]


def write_json(report: dict[str, Any], output: Path | None) -> None:
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if output is None:
        sys.stdout.write(text)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner", type=Path, default=Path("target/release/ann_corpus_check"))
    parser.add_argument("--vectors", type=Path, required=True)
    parser.add_argument("--queries", type=Path, required=True)
    parser.add_argument("--ground-truth", type=Path, required=True)
    parser.add_argument("--metric", default="dot_product")
    parser.add_argument("--iterations", type=positive_int, default=3)
    parser.add_argument("--min-recall-q16", type=int, default=65_535)
    parser.add_argument("--min-mean-recall-q16", type=int, default=65_535)
    parser.add_argument("--max-neighbors", type=positive_int, default=8)
    parser.add_argument("--ef-search", type=positive_int, default=64)
    parser.add_argument("--ef-construction", type=positive_int, default=64)
    parser.add_argument("--layer-count", type=positive_int, default=4)
    parser.add_argument("--max-p95-latency-nanos", type=int, default=100_000_000)
    parser.add_argument("--max-p99-latency-nanos", type=int, default=200_000_000)
    parser.add_argument("--max-max-latency-nanos", type=int, default=250_000_000)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    report = run_probe(args)
    write_json(report, args.output)
    return 0 if report["passed"] else 1


class SelfTests(unittest.TestCase):
    def report(self, recall: int = 65_535, p99: int = 10) -> dict[str, Any]:
        return {
            "passed": True,
            "production_safe": True,
            "vector_count": 3,
            "query_count": 1,
            "dimension": 2,
            "graph_nodes": 3,
            "graph_edges": 4,
            "upper_layers": 1,
            "upper_graph_edges": 1,
            "min_observed_recall_q16": recall,
            "mean_recall_q16": recall,
            "p95_latency_nanos": p99,
            "p99_latency_nanos": p99,
            "max_latency_nanos": p99,
        }

    def test_summary_accepts_stable_safe_reports(self) -> None:
        report = summarize_reports([self.report(), self.report()], [], 2)
        self.assertTrue(report["passed"])
        self.assertTrue(report["production_safe"])
        self.assertEqual(report["iterations"], 2)

    def test_summary_rejects_changed_graph_shape(self) -> None:
        second = self.report()
        second["graph_edges"] = 3
        report = summarize_reports([self.report(), second], [], 2)
        self.assertFalse(report["passed"])
        self.assertTrue(any("graph shape changed" in item for item in report["failures"]))

    def test_summary_rejects_missing_iterations(self) -> None:
        report = summarize_reports([self.report()], [], 2)
        self.assertFalse(report["passed"])
        self.assertTrue(any("expected 2 probe" in item for item in report["failures"]))

    def test_summary_rejects_low_recall(self) -> None:
        report = summarize_reports([self.report(recall=40_000)], [], 1)
        self.assertFalse(report["passed"])
        self.assertTrue(any("below probe threshold" in item for item in report["failures"]))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
