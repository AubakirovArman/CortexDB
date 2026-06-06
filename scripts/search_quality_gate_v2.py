#!/usr/bin/env python3
"""Release gate for search quality v2 evidence."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def int_value(row: dict[str, Any], field: str) -> int:
    value = row.get(field)
    if not isinstance(value, int):
        raise ValueError(f"{field}: expected integer")
    return value


def check_min(name: str, actual: int, minimum: int, failures: list[str]) -> None:
    if actual < minimum:
        failures.append(f"{name} below threshold: {actual} < {minimum}")


def check_max(name: str, actual: int, maximum: int, failures: list[str]) -> None:
    if actual > maximum:
        failures.append(f"{name} above threshold: {actual} > {maximum}")


def domain_by_name(beta: dict[str, Any]) -> dict[str, dict[str, Any]]:
    domains = beta.get("domains")
    if not isinstance(domains, list):
        raise ValueError("beta report domains must be a list")
    output = {}
    for row in domains:
        if isinstance(row, dict) and isinstance(row.get("domain"), str):
            output[row["domain"]] = row
    return output


def validate_domain_thresholds(
    beta: dict[str, Any],
    thresholds: dict[str, Any],
    failures: list[str],
) -> list[dict[str, Any]]:
    rows = domain_by_name(beta)
    domain_thresholds = thresholds.get("domains")
    if not isinstance(domain_thresholds, dict) or not domain_thresholds:
        raise ValueError("thresholds.domains must be a non-empty object")
    checks = []
    for domain, threshold in sorted(domain_thresholds.items()):
        if not isinstance(threshold, dict):
            raise ValueError(f"{domain}: threshold must be an object")
        row = rows.get(domain)
        if row is None:
            failures.append(f"{domain}: missing beta domain report")
            continue
        check_min(f"{domain}.recall", int_value(row, "latest_mean_recall_q16"), int_value(threshold, "min_recall_q16"), failures)
        check_min(f"{domain}.mrr", int_value(row, "latest_mean_mrr_q16"), int_value(threshold, "min_mrr_q16"), failures)
        check_min(f"{domain}.ndcg", int_value(row, "latest_mean_ndcg_q16"), int_value(threshold, "min_ndcg_q16"), failures)
        check_min(f"{domain}.exact_parity", int_value(row, "latest_exact_parity_q16"), int_value(threshold, "min_exact_parity_q16"), failures)
        check_max(f"{domain}.p95_latency", int_value(row, "latest_p95_latency_nanos"), int_value(threshold, "max_p95_latency_nanos"), failures)
        if int_value(row, "regression_count") != 0:
            failures.append(f"{domain}: beta report has regression_count={row['regression_count']}")
        checks.append({"domain": domain, "status": "checked", "threshold": threshold, "observed": row})
    return checks


def validate_history(history: dict[str, Any], failures: list[str]) -> dict[str, Any]:
    if history.get("status") != "passed":
        failures.append(f"history status is not passed: {history.get('status')}")
    if history.get("production_safe") is not True:
        failures.append("history production_safe must be true")
    if int_value(history, "regression_count") != 0:
        failures.append(f"history has regression_count={history['regression_count']}")
    for row in history.get("domains", []):
        if isinstance(row, dict) and int_value(row, "regression_count") != 0:
            failures.append(f"{row.get('domain', '<unknown>')}: history regression_count={row['regression_count']}")
    return {
        "status": history.get("status"),
        "production_safe": history.get("production_safe"),
        "regression_count": history.get("regression_count"),
        "domain_count": history.get("domain_count"),
    }


def validate_ann_safe_mode(
    ann_history: dict[str, Any],
    thresholds: dict[str, Any],
    failures: list[str],
) -> list[dict[str, Any]]:
    policy = thresholds.get("ann_safe_mode")
    if not isinstance(policy, dict):
        raise ValueError("thresholds.ann_safe_mode must be an object")
    if int_value(ann_history, "regression_count") != 0:
        failures.append(f"ANN history has regression_count={ann_history['regression_count']}")
    corpora = ann_history.get("corpora")
    if not isinstance(corpora, list) or not corpora:
        failures.append("ANN history has no corpora")
        return []
    checks = []
    for index, corpus in enumerate(corpora):
        if not isinstance(corpus, dict):
            failures.append(f"ANN corpus[{index}] is not an object")
            continue
        name = f"ann.corpus[{index}]"
        if policy.get("require_production_safe") is True and corpus.get("latest_production_safe") is not True:
            failures.append(f"{name}.latest_production_safe must be true")
        check_min(f"{name}.mean_recall", int_value(corpus, "latest_mean_recall_q16"), int_value(policy, "min_mean_recall_q16"), failures)
        check_min(f"{name}.exact_parity", int_value(corpus, "latest_exact_parity_q16"), int_value(policy, "min_exact_parity_q16"), failures)
        check_min(f"{name}.graph_freshness", int_value(corpus, "latest_graph_freshness_q16"), int_value(policy, "min_graph_freshness_q16"), failures)
        check_max(f"{name}.fallback_count", int_value(corpus, "latest_fallback_count"), int_value(policy, "max_fallback_count"), failures)
        check_max(f"{name}.fallback_rate", int_value(corpus, "latest_fallback_rate_q16"), int_value(policy, "max_fallback_rate_q16"), failures)
        check_max(f"{name}.stale_vector_count", int_value(corpus, "latest_stale_vector_count"), int_value(policy, "max_stale_vector_count"), failures)
        checks.append({"corpus_key": corpus.get("corpus_key"), "status": "checked"})
    return checks


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    thresholds = load_json(args.thresholds)
    beta = load_json(args.beta_report)
    history = load_json(args.history_report)
    ann_history = load_json(args.ann_history)
    failures: list[str] = []
    if beta.get("status") != "passed":
        failures.append(f"beta status is not passed: {beta.get('status')}")
    if beta.get("production_safe") is not True:
        failures.append("beta production_safe must be true")
    domain_checks = validate_domain_thresholds(beta, thresholds, failures)
    history_check = validate_history(history, failures)
    ann_checks = validate_ann_safe_mode(ann_history, thresholds, failures)
    return {
        "schema_version": "cortexdb.search_quality_gate_v2.report.v1",
        "status": "passed" if not failures else "failed",
        "production_safe": not failures,
        "failures": failures,
        "release_gate": {"fail_on_regression": True, "passed": not failures},
        "domain_checks": domain_checks,
        "history_check": history_check,
        "ann_safe_mode": {"status": "checked", "corpora": ann_checks},
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--thresholds", type=Path, required=True)
    parser.add_argument("--beta-report", type=Path, required=True)
    parser.add_argument("--history-report", type=Path, required=True)
    parser.add_argument("--ann-history", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def run(args: argparse.Namespace) -> int:
    report = build_report(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"search quality gate v2 passed: {args.output}")
    return 0


class SelfTests(unittest.TestCase):
    def write_json(self, root: Path, name: str, value: dict[str, Any]) -> Path:
        path = root / name
        path.write_text(json.dumps(value), encoding="utf-8")
        return path

    def fixture(self) -> dict[str, Any]:
        return {
            "thresholds": {
                "domains": {"demo": {"min_recall_q16": 10, "min_mrr_q16": 10, "min_ndcg_q16": 10, "min_exact_parity_q16": 65535, "max_p95_latency_nanos": 100}},
                "ann_safe_mode": {"require_production_safe": True, "min_mean_recall_q16": 10, "min_exact_parity_q16": 65535, "min_graph_freshness_q16": 65535, "max_fallback_count": 0, "max_fallback_rate_q16": 0, "max_stale_vector_count": 0},
            },
            "beta": {"status": "passed", "production_safe": True, "domains": [{"domain": "demo", "latest_mean_recall_q16": 10, "latest_mean_mrr_q16": 10, "latest_mean_ndcg_q16": 10, "latest_exact_parity_q16": 65535, "latest_p95_latency_nanos": 10, "regression_count": 0}]},
            "history": {"status": "passed", "production_safe": True, "regression_count": 0, "domain_count": 1, "domains": [{"domain": "demo", "regression_count": 0}]},
            "ann": {"regression_count": 0, "corpora": [{"corpus_key": "demo", "latest_production_safe": True, "latest_mean_recall_q16": 10, "latest_exact_parity_q16": 65535, "latest_graph_freshness_q16": 65535, "latest_fallback_count": 0, "latest_fallback_rate_q16": 0, "latest_stale_vector_count": 0}]},
        }

    def build(self, fixture: dict[str, Any]) -> dict[str, Any]:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            args = argparse.Namespace(
                thresholds=self.write_json(root, "thresholds.json", fixture["thresholds"]),
                beta_report=self.write_json(root, "beta.json", fixture["beta"]),
                history_report=self.write_json(root, "history.json", fixture["history"]),
                ann_history=self.write_json(root, "ann.json", fixture["ann"]),
                output=root / "report.json",
            )
            return build_report(args)

    def test_pass_path(self) -> None:
        self.assertEqual(self.build(self.fixture())["status"], "passed")

    def test_threshold_failure_fails_gate(self) -> None:
        fixture = self.fixture()
        fixture["beta"]["domains"][0]["latest_mean_recall_q16"] = 9
        self.assertEqual(self.build(fixture)["status"], "failed")

    def test_ann_fallback_fails_safe_mode(self) -> None:
        fixture = self.fixture()
        fixture["ann"]["corpora"][0]["latest_fallback_count"] = 1
        self.assertEqual(self.build(fixture)["status"], "failed")

    def test_history_regression_fails_release_gate(self) -> None:
        fixture = self.fixture()
        fixture["history"]["regression_count"] = 1
        self.assertEqual(self.build(fixture)["release_gate"]["passed"], False)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    return run(parse_args(argv))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
