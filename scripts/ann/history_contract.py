#!/usr/bin/env python3
"""Validate ANN history summaries carried inside release artifacts."""

from __future__ import annotations

import json
import sys
import unittest
from typing import Any


def validate_packaged_history(
    history: dict[str, Any],
    *,
    source_run_id: str,
    min_runs: int = 1,
    min_corpora: int = 1,
) -> None:
    failures = packaged_history_failures(
        history,
        source_run_id=source_run_id,
        min_runs=min_runs,
        min_corpora=min_corpora,
    )
    if failures:
        raise ValueError("; ".join(failures))


def packaged_history_failures(
    history: dict[str, Any],
    *,
    source_run_id: str,
    min_runs: int,
    min_corpora: int,
) -> list[str]:
    failures: list[str] = []
    run_count = int_field(history, "run_count", failures)
    corpus_count = int_field(history, "corpus_count", failures)
    regression_count = int_field(history, "regression_count", failures)
    if run_count is not None and run_count < min_runs:
        failures.append(f"history.json: expected at least {min_runs} run(s), found {run_count}")
    if corpus_count is not None and corpus_count < min_corpora:
        failures.append(f"history.json: expected at least {min_corpora} corpus group(s), found {corpus_count}")
    if regression_count is not None and regression_count != 0:
        failures.append(f"history.json: expected zero regressions, found {regression_count}")
    runs = list_field(history, "runs", failures)
    if runs is not None and source_run_id and not has_run_id(runs, source_run_id):
        failures.append(f"history.json: source run '{source_run_id}' not found")
    corpora = list_field(history, "corpora", failures)
    if corpora is not None:
        for index, corpus in enumerate(corpora):
            if not isinstance(corpus, dict):
                failures.append(f"history.json:corpora[{index}]: expected object")
                continue
            if corpus.get("latest_production_safe") is not True:
                failures.append(f"history.json:corpora[{index}]: latest_production_safe must be true")
            if int(corpus.get("run_count", 0)) <= 0:
                failures.append(f"history.json:corpora[{index}]: run_count must be positive")
            if int(corpus.get("latest_fallback_rate_q16", 0)) != 0:
                failures.append(f"history.json:corpora[{index}]: latest_fallback_rate_q16 must be 0")
            if int(corpus.get("latest_fallback_count", 0)) != 0:
                failures.append(f"history.json:corpora[{index}]: latest_fallback_count must be 0")
            if int(corpus.get("latest_graph_freshness_q16", 65_535)) < 65_535:
                failures.append(f"history.json:corpora[{index}]: latest_graph_freshness_q16 must be 65535")
            if int(corpus.get("latest_stale_vector_count", 0)) != 0:
                failures.append(f"history.json:corpora[{index}]: latest_stale_vector_count must be 0")
    return failures


def int_field(value: dict[str, Any], field: str, failures: list[str]) -> int | None:
    raw = value.get(field)
    if not isinstance(raw, int):
        failures.append(f"history.json:{field}: expected integer")
        return None
    return raw


def list_field(value: dict[str, Any], field: str, failures: list[str]) -> list[Any] | None:
    raw = value.get(field)
    if not isinstance(raw, list):
        failures.append(f"history.json:{field}: expected list")
        return None
    return raw


def has_run_id(runs: list[Any], run_id: str) -> bool:
    return any(isinstance(run, dict) and run.get("run_id") == run_id for run in runs)


def main(argv: list[str]) -> int:
    if argv == ["--self-test"]:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    if len(argv) != 2:
        print("usage: history_contract.py HISTORY_JSON SOURCE_RUN_ID", file=sys.stderr)
        return 2
    with open(argv[0], "r", encoding="utf-8") as handle:
        history = json.load(handle)
    validate_packaged_history(history, source_run_id=argv[1])
    print(json.dumps({"passed": True}, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def history(self) -> dict[str, Any]:
        return {
            "run_count": 1,
            "corpus_count": 1,
            "regression_count": 0,
            "runs": [{"run_id": "smoke", "production_safe": True}],
            "corpora": [{
                "run_count": 1,
                "latest_run_id": "smoke",
                "latest_production_safe": True,
                "latest_fallback_count": 0,
                "latest_fallback_rate_q16": 0,
                "latest_graph_freshness_q16": 65_535,
                "latest_stale_vector_count": 0,
            }],
        }

    def test_clean_history_passes(self) -> None:
        validate_packaged_history(self.history(), source_run_id="smoke")

    def test_regression_fails(self) -> None:
        history = self.history()
        history["regression_count"] = 1
        with self.assertRaises(ValueError):
            validate_packaged_history(history, source_run_id="smoke")

    def test_missing_source_run_fails(self) -> None:
        with self.assertRaises(ValueError):
            validate_packaged_history(self.history(), source_run_id="missing")

    def test_latest_unsafe_corpus_fails(self) -> None:
        history = self.history()
        history["corpora"][0]["latest_production_safe"] = False
        with self.assertRaises(ValueError):
            validate_packaged_history(history, source_run_id="smoke")

    def test_latest_fallback_or_stale_graph_fails(self) -> None:
        history = self.history()
        history["corpora"][0]["latest_fallback_count"] = 1
        history["corpora"][0]["latest_fallback_rate_q16"] = 1
        history["corpora"][0]["latest_graph_freshness_q16"] = 65_534
        history["corpora"][0]["latest_stale_vector_count"] = 1
        with self.assertRaises(ValueError):
            validate_packaged_history(history, source_run_id="smoke")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
