#!/usr/bin/env python3
"""Materialize checked-in ANN history fixtures for clean release gates."""

from __future__ import annotations

import argparse
import json
import tempfile
import unittest
from pathlib import Path
from typing import Any


REPORT_FIELDS = (
    "passed",
    "production_safe",
    "metric",
    "vector_count",
    "query_count",
    "dimension",
    "hnsw_max_neighbors",
    "hnsw_ef_search",
    "hnsw_layer_count",
    "graph_nodes",
    "graph_edges",
    "upper_layers",
    "upper_graph_edges",
    "graph_freshness_q16",
    "stale_vector_count",
    "fallback_count",
    "fallback_rate_q16",
    "min_observed_recall_q16",
    "mean_recall_q16",
    "mean_mrr_q16",
    "mean_ndcg_q16",
    "exact_parity_q16",
    "exact_parity_count",
    "p50_latency_nanos",
    "p95_latency_nanos",
    "p99_latency_nanos",
    "max_latency_nanos",
)


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def existing_run_count(run_root: Path) -> int:
    return len(list(run_root.glob("*/manifest.json")))


def int_value(run: dict[str, Any], field: str, default: int) -> int:
    value = run.get(field, default)
    if not isinstance(value, int):
        raise ValueError(f"{run.get('run_id', '<unknown>')}:{field}: expected int")
    return value


def str_value(run: dict[str, Any], field: str, default: str = "") -> str:
    value = run.get(field, default)
    if not isinstance(value, str):
        raise ValueError(f"{run.get('run_id', '<unknown>')}:{field}: expected string")
    return value


def bool_value(run: dict[str, Any], field: str) -> bool:
    value = run.get(field)
    if not isinstance(value, bool):
        raise ValueError(f"{run.get('run_id', '<unknown>')}:{field}: expected bool")
    return value


def query_rows(run: dict[str, Any]) -> list[dict[str, Any]]:
    rows = []
    query_count = int_value(run, "query_count", 1)
    for index in range(query_count):
        rows.append(
            {
                "name": f"{run['run_id']}-q{index + 1}",
                "recall_q16": int_value(run, "mean_recall_q16", 65_535),
                "reciprocal_rank_q16": int_value(run, "mean_mrr_q16", 65_535),
                "ndcg_q16": int_value(run, "mean_ndcg_q16", 65_535),
                "exact_parity": True,
                "latency_nanos": int_value(run, "p50_latency_nanos", 1),
                "production_safe": bool_value(run, "production_safe"),
            }
        )
    return rows


def materialize_run(run_root: Path, run: dict[str, Any]) -> None:
    run_id = str_value(run, "run_id")
    run_dir = run_root / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "run_id": run_id,
        "git_sha": str_value(run, "git_sha"),
        "metric": str_value(run, "metric"),
        "vectors": str_value(run, "vectors"),
        "queries": str_value(run, "queries"),
        "ground_truth": str_value(run, "ground_truth"),
        "machine_profile": str_value(run, "machine_profile", "machine_profile.json"),
        "baseline_report": str_value(run, "baseline_report"),
        "report": str(run_dir / "report.json"),
    }
    report = {field: run[field] for field in REPORT_FIELDS if field in run}
    report.setdefault("graph_freshness_q16", 65_535)
    report.setdefault("stale_vector_count", 0)
    report.setdefault("fallback_count", 0)
    report.setdefault("fallback_rate_q16", 0)
    report.setdefault("mean_mrr_q16", 65_535)
    report.setdefault("mean_ndcg_q16", 65_535)
    report.setdefault("exact_parity_q16", 65_535)
    report.setdefault("exact_parity_count", int_value(run, "query_count", 1))
    report["queries"] = query_rows({**run, **report})
    (run_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    (run_dir / "report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def materialize(source: Path, run_root: Path, min_runs: int) -> dict[str, Any]:
    current_runs = existing_run_count(run_root)
    if current_runs:
        return {"status": "skipped", "existing_runs": current_runs, "run_root": str(run_root)}
    history = load_json(source)
    runs = history.get("runs")
    if not isinstance(runs, list) or len(runs) < min_runs:
        raise ValueError(f"{source}: expected at least {min_runs} fixture run(s)")
    for run in runs:
        if not isinstance(run, dict):
            raise ValueError(f"{source}: run entries must be objects")
        materialize_run(run_root, run)
    return {"status": "materialized", "runs": len(runs), "run_root": str(run_root)}


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source",
        type=Path,
        default=Path("crates/cortex-engine/fixtures/ann_history_clean_v1.json"),
    )
    parser.add_argument("--run-root", type=Path, default=Path("target/ann/real-embedding/runs"))
    parser.add_argument("--min-runs", type=int, default=3)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    result = materialize(args.source, args.run_root, args.min_runs)
    print(json.dumps(result, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def test_materialize_writes_manifest_and_query_rows(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            source = root / "history.json"
            source.write_text(
                json.dumps({"runs": [fixture_run("a", 10), fixture_run("b", 9), fixture_run("c", 8)]}),
                encoding="utf-8",
            )
            run_root = root / "runs"
            result = materialize(source, run_root, 3)
            self.assertEqual(result["status"], "materialized")
            report = load_json(run_root / "c" / "report.json")
            self.assertEqual(len(report["queries"]), 4)
            self.assertEqual(existing_run_count(run_root), 3)

    def test_existing_history_is_left_untouched(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            run_root = root / "runs"
            materialize_run(run_root, fixture_run("existing", 1))
            source = root / "history.json"
            source.write_text(json.dumps({"runs": []}), encoding="utf-8")
            result = materialize(source, run_root, 3)
            self.assertEqual(result["status"], "skipped")
            self.assertEqual(result["existing_runs"], 1)


def fixture_run(run_id: str, latency: int) -> dict[str, Any]:
    return {
        "run_id": run_id,
        "git_sha": f"sha-{run_id}",
        "metric": "dot_product",
        "vectors": "vectors.jsonl",
        "queries": "queries.jsonl",
        "ground_truth": "ground_truth.jsonl",
        "machine_profile": "machine_profile.json",
        "baseline_report": "",
        "passed": True,
        "production_safe": True,
        "vector_count": 12,
        "query_count": 4,
        "dimension": 8,
        "hnsw_max_neighbors": 8,
        "hnsw_ef_search": 64,
        "hnsw_layer_count": 4,
        "graph_nodes": 12,
        "graph_edges": 120,
        "upper_layers": 3,
        "upper_graph_edges": 2,
        "min_observed_recall_q16": 65_535,
        "mean_recall_q16": 65_535,
        "mean_mrr_q16": 65_535,
        "mean_ndcg_q16": 65_535,
        "exact_parity_q16": 65_535,
        "p50_latency_nanos": latency,
        "p95_latency_nanos": latency,
        "p99_latency_nanos": latency,
        "max_latency_nanos": latency,
    }


if __name__ == "__main__":
    raise SystemExit(main())
