#!/usr/bin/env python3
"""Validate ANN report fields required for release-grade evidence."""

from __future__ import annotations

import json
import sys
import unittest
from typing import Any


MIN_POLICY_FIELDS = [
    "required_min_recall_q16",
    "required_min_mean_recall_q16",
    "allowed_p95_latency_nanos",
    "allowed_p99_latency_nanos",
    "allowed_max_latency_nanos",
]


def int_field(report: dict[str, Any], field: str) -> int:
    value = report.get(field)
    if not isinstance(value, int):
        raise ValueError(f"report.json:{field}: expected integer")
    return value


def bool_field(report: dict[str, Any], field: str) -> bool:
    value = report.get(field)
    if not isinstance(value, bool):
        raise ValueError(f"report.json:{field}: expected boolean")
    return value


def validate_production_report(report: dict[str, Any]) -> None:
    failures = production_report_failures(report)
    if failures:
        raise ValueError("; ".join(failures))


def production_report_failures(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    try:
        for field in MIN_POLICY_FIELDS:
            if int_field(report, field) <= 0:
                failures.append(f"report.json:{field}: expected > 0")
        if not bool_field(report, "require_production_safe"):
            failures.append("report.json:require_production_safe must be true")
        if not bool_field(report, "passed"):
            failures.append("report.json:passed must be true")
        if not bool_field(report, "production_safe"):
            failures.append("report.json:production_safe must be true")
        if int_field(report, "hnsw_layer_count") <= 1:
            failures.append("report.json:hnsw_layer_count must be multi-layer")
        if int_field(report, "upper_layers") <= 0:
            failures.append("report.json:upper_layers must be > 0")
        if int_field(report, "upper_graph_edges") <= 0:
            failures.append("report.json:upper_graph_edges must be > 0")
        if int_field(report, "min_observed_recall_q16") < int_field(report, "required_min_recall_q16"):
            failures.append("report.json:min_observed_recall_q16 below required minimum")
        if int_field(report, "mean_recall_q16") < int_field(report, "required_min_mean_recall_q16"):
            failures.append("report.json:mean_recall_q16 below required minimum")
        if int_field(report, "p95_latency_nanos") > int_field(report, "allowed_p95_latency_nanos"):
            failures.append("report.json:p95_latency_nanos exceeds allowed maximum")
        if int_field(report, "p99_latency_nanos") > int_field(
            report, "allowed_p99_latency_nanos"
        ):
            failures.append("report.json:p99_latency_nanos exceeds allowed maximum")
        if int_field(report, "max_latency_nanos") > int_field(report, "allowed_max_latency_nanos"):
            failures.append("report.json:max_latency_nanos exceeds allowed maximum")
    except ValueError as error:
        failures.append(str(error))
    return failures


def compare_gate_policy(baseline: dict[str, Any], candidate: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    for field in ["required_min_recall_q16", "required_min_mean_recall_q16"]:
        if field in baseline and field not in candidate:
            failures.append(f"{field} missing from candidate report")
        elif field in baseline and int(candidate[field]) < int(baseline[field]):
            failures.append(f"{field} relaxed: {baseline[field]} -> {candidate[field]}")
    for field in [
        "allowed_p95_latency_nanos",
        "allowed_p99_latency_nanos",
        "allowed_max_latency_nanos",
    ]:
        if field in baseline and field not in candidate:
            failures.append(f"{field} missing from candidate report")
        elif field in baseline and int(candidate[field]) > int(baseline[field]):
            failures.append(f"{field} relaxed: {baseline[field]} -> {candidate[field]}")
    if baseline.get("require_production_safe") is True and candidate.get("require_production_safe") is not True:
        failures.append("require_production_safe relaxed from true")
    return failures


def main(argv: list[str]) -> int:
    if argv == ["--self-test"]:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    if len(argv) != 1:
        print("usage: report_contract.py REPORT_JSON", file=sys.stderr)
        return 2
    with open(argv[0], "r", encoding="utf-8") as handle:
        report = json.load(handle)
    validate_production_report(report)
    print(json.dumps({"passed": True}, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def report(self) -> dict[str, Any]:
        return {
            "passed": True,
            "production_safe": True,
            "require_production_safe": True,
            "required_min_recall_q16": 49_151,
            "required_min_mean_recall_q16": 49_151,
            "allowed_p95_latency_nanos": 100,
            "allowed_p99_latency_nanos": 150,
            "allowed_max_latency_nanos": 200,
            "hnsw_layer_count": 4,
            "upper_layers": 2,
            "upper_graph_edges": 3,
            "min_observed_recall_q16": 65_535,
            "mean_recall_q16": 65_535,
            "p95_latency_nanos": 50,
            "p99_latency_nanos": 60,
            "max_latency_nanos": 75,
        }

    def test_valid_production_report_passes(self) -> None:
        validate_production_report(self.report())

    def test_single_layer_report_fails(self) -> None:
        report = self.report()
        report["upper_layers"] = 0
        with self.assertRaises(ValueError):
            validate_production_report(report)

    def test_compare_rejects_relaxed_recall_policy(self) -> None:
        baseline = self.report()
        candidate = self.report()
        candidate["required_min_recall_q16"] = 10
        self.assertTrue(any("relaxed" in failure for failure in compare_gate_policy(baseline, candidate)))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
