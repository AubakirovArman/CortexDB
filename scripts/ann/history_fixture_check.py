#!/usr/bin/env python3
"""Validate checked-in ANN history fixtures.

The fixtures make the release gate self-checking: one history must pass, while
recall and latency regression histories must fail for the expected reason.
"""

from __future__ import annotations

import argparse
import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

from history_contract import validate_packaged_history


DEFAULT_CLEAN = Path("crates/cortex-engine/fixtures/ann_history_clean_v1.json")
DEFAULT_RECALL_REGRESSION = Path(
    "crates/cortex-engine/fixtures/ann_history_recall_regression_v1.json"
)
DEFAULT_LATENCY_REGRESSION = Path(
    "crates/cortex-engine/fixtures/ann_history_latency_regression_v1.json"
)
RECALL_FIELDS = {"min_observed_recall_q16", "mean_recall_q16"}
LATENCY_FIELDS = {"p95_latency_nanos", "p99_latency_nanos", "max_latency_nanos"}


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def latest_run_id(history: dict[str, Any]) -> str:
    latest = history.get("latest_run_id")
    if isinstance(latest, str) and latest:
        return latest
    runs = history.get("runs")
    if isinstance(runs, list) and runs:
        last = runs[-1]
        if isinstance(last, dict) and isinstance(last.get("run_id"), str):
            return str(last["run_id"])
    raise ValueError("history fixture has no latest run id")


def validate_clean_history(history: dict[str, Any]) -> None:
    validate_packaged_history(
        history,
        source_run_id=latest_run_id(history),
        min_runs=2,
        min_corpora=1,
    )
    regressions = history.get("regressions", [])
    if regressions:
        raise ValueError("clean fixture must not contain regressions")
    runs = history.get("runs", [])
    if not isinstance(runs, list):
        raise ValueError("clean fixture runs must be a list")
    if not all(isinstance(run, dict) and run.get("production_safe") is True for run in runs):
        raise ValueError("clean fixture runs must all be production_safe")
    has_multilayer_evidence = any(
        has_positive_int(run, "upper_layers") and has_positive_int(run, "upper_graph_edges")
        for run in runs
    )
    if not has_multilayer_evidence:
        raise ValueError("clean fixture must include multi-layer graph evidence")


def validate_expected_regression(history: dict[str, Any], *, kind: str, fields: set[str]) -> None:
    try:
        validate_packaged_history(
            history,
            source_run_id=latest_run_id(history),
            min_runs=2,
            min_corpora=1,
        )
    except ValueError:
        pass
    else:
        raise ValueError(f"{kind} fixture unexpectedly passed packaged history validation")
    regressions = collect_regressions(history)
    has_expected_regression = any(
        regression.get("kind") == kind and regression.get("field") in fields
        for regression in regressions
    )
    if not has_expected_regression:
        expected = ", ".join(sorted(fields))
        raise ValueError(f"{kind} fixture must contain {kind} regression for one of: {expected}")


def collect_regressions(history: dict[str, Any]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    raw_regressions = history.get("regressions", [])
    if isinstance(raw_regressions, list):
        result.extend(item for item in raw_regressions if isinstance(item, dict))
    raw_corpora = history.get("corpora", [])
    if isinstance(raw_corpora, list):
        for corpus in raw_corpora:
            if not isinstance(corpus, dict):
                continue
            nested = corpus.get("regressions", [])
            if isinstance(nested, list):
                result.extend(item for item in nested if isinstance(item, dict))
    return result


def has_positive_int(value: dict[str, Any], field: str) -> bool:
    return isinstance(value.get(field), int) and int(value[field]) > 0


def run_check(clean_path: Path, recall_path: Path, latency_path: Path) -> dict[str, Any]:
    validate_clean_history(load_json(clean_path))
    validate_expected_regression(load_json(recall_path), kind="recall", fields=RECALL_FIELDS)
    validate_expected_regression(load_json(latency_path), kind="latency", fields=LATENCY_FIELDS)
    return {
        "passed": True,
        "clean_fixture": str(clean_path),
        "recall_regression_fixture": str(recall_path),
        "latency_regression_fixture": str(latency_path),
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="validate ANN history fixtures")
    parser.add_argument("--clean", type=Path, default=DEFAULT_CLEAN)
    parser.add_argument("--recall-regression", type=Path, default=DEFAULT_RECALL_REGRESSION)
    parser.add_argument("--latency-regression", type=Path, default=DEFAULT_LATENCY_REGRESSION)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    result = run_check(args.clean, args.recall_regression, args.latency_regression)
    print(json.dumps(result, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def write_history(self, root: Path, name: str, history: dict[str, Any]) -> Path:
        path = root / name
        path.write_text(json.dumps(history), encoding="utf-8")
        return path

    def clean_history(self) -> dict[str, Any]:
        return {
            "run_count": 2,
            "corpus_count": 1,
            "regression_count": 0,
            "latest_run_id": "b",
            "regressions": [],
            "runs": [
                {"run_id": "a", "production_safe": True, "upper_layers": 2, "upper_graph_edges": 1},
                {"run_id": "b", "production_safe": True, "upper_layers": 2, "upper_graph_edges": 1},
            ],
            "corpora": [{"run_count": 2, "latest_run_id": "b", "latest_production_safe": True}],
        }

    def regression_history(self, kind: str, field: str) -> dict[str, Any]:
        history = self.clean_history()
        history["regression_count"] = 1
        history["regressions"] = [{"kind": kind, "field": field}]
        return history

    def test_run_check_accepts_expected_fixtures(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            clean = self.write_history(root, "clean.json", self.clean_history())
            recall = self.write_history(
                root,
                "recall.json",
                self.regression_history("recall", "mean_recall_q16"),
            )
            latency = self.write_history(
                root,
                "latency.json",
                self.regression_history("latency", "p95_latency_nanos"),
            )
            self.assertTrue(run_check(clean, recall, latency)["passed"])

    def test_clean_fixture_rejects_regression(self) -> None:
        with self.assertRaises(ValueError):
            validate_clean_history(self.regression_history("recall", "mean_recall_q16"))

    def test_regression_fixture_rejects_wrong_kind(self) -> None:
        with self.assertRaises(ValueError):
            validate_expected_regression(
                self.regression_history("latency", "p95_latency_nanos"),
                kind="recall",
                fields=RECALL_FIELDS,
            )


if __name__ == "__main__":
    raise SystemExit(main())
