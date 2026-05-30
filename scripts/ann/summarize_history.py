#!/usr/bin/env python3
"""Summarize archived ANN corpus runs and adjacent regressions."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def as_int(value: Any, field: str) -> int:
    if not isinstance(value, int):
        raise ValueError(f"{field}: expected integer")
    return value


def as_int_default(value: Any, default: int) -> int: return value if isinstance(value, int) else default


def as_bool(value: Any, field: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{field}: expected boolean")
    return value


def as_str(value: Any, field: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{field}: expected string")
    return value


def resolve_report_path(run_dir: Path, manifest: dict[str, Any]) -> Path:
    report_value = manifest.get("report")
    if isinstance(report_value, str) and report_value:
        report_path = Path(report_value)
        if report_path.exists():
            return report_path
        candidate = run_dir / report_path.name
        if candidate.exists():
            return candidate
    return run_dir / "report.json"


def corpus_key(run: dict[str, Any]) -> str:
    return "|".join([run["metric"], str(run["dimension"]), run["vectors"], run["queries"]])


def load_run(manifest_path: Path) -> dict[str, Any]:
    run_dir = manifest_path.parent
    manifest = load_json(manifest_path)
    report_path = resolve_report_path(run_dir, manifest)
    report = load_json(report_path)
    comparison_path = run_dir / "comparison.json"
    comparison = load_json(comparison_path) if comparison_path.exists() else None
    run_id = as_str(manifest.get("run_id", run_dir.name), f"{manifest_path}:run_id")
    metric = as_str(report.get("metric", manifest.get("metric")), f"{report_path}:metric")
    run = {
        "run_id": run_id,
        "git_sha": as_str(manifest.get("git_sha", ""), f"{manifest_path}:git_sha"),
        "metric": metric,
        "vectors": as_str(manifest.get("vectors", ""), f"{manifest_path}:vectors"),
        "queries": as_str(manifest.get("queries", ""), f"{manifest_path}:queries"),
        "ground_truth": as_str(
            manifest.get("ground_truth", ""),
            f"{manifest_path}:ground_truth",
        ),
        "baseline_report": as_str(
            manifest.get("baseline_report", ""),
            f"{manifest_path}:baseline_report",
        ),
        "report": str(report_path),
        "comparison": str(comparison_path) if comparison is not None else "",
        "passed": as_bool(report.get("passed"), f"{report_path}:passed"),
        "production_safe": as_bool(
            report.get("production_safe"),
            f"{report_path}:production_safe",
        ),
        "vector_count": as_int(report.get("vector_count"), f"{report_path}:vector_count"),
        "query_count": as_int(report.get("query_count"), f"{report_path}:query_count"),
        "dimension": as_int(report.get("dimension"), f"{report_path}:dimension"),
        "hnsw_max_neighbors": as_int_default(report.get("hnsw_max_neighbors"), 8),
        "hnsw_ef_search": as_int_default(report.get("hnsw_ef_search"), 64),
        "hnsw_layer_count": as_int_default(report.get("hnsw_layer_count"), 4),
        "graph_nodes": as_int(report.get("graph_nodes"), f"{report_path}:graph_nodes"),
        "graph_edges": as_int(report.get("graph_edges"), f"{report_path}:graph_edges"),
        "upper_layers": as_int(report.get("upper_layers"), f"{report_path}:upper_layers"),
        "upper_graph_edges": as_int(
            report.get("upper_graph_edges"),
            f"{report_path}:upper_graph_edges",
        ),
        "min_observed_recall_q16": as_int(
            report.get("min_observed_recall_q16"),
            f"{report_path}:min_observed_recall_q16",
        ),
        "mean_recall_q16": as_int(
            report.get("mean_recall_q16"),
            f"{report_path}:mean_recall_q16",
        ),
        "p50_latency_nanos": as_int(
            report.get("p50_latency_nanos"),
            f"{report_path}:p50_latency_nanos",
        ),
        "p95_latency_nanos": as_int(
            report.get("p95_latency_nanos"),
            f"{report_path}:p95_latency_nanos",
        ),
        "max_latency_nanos": as_int(
            report.get("max_latency_nanos"),
            f"{report_path}:max_latency_nanos",
        ),
    }
    run["corpus_key"] = corpus_key(run)
    return run


def compare_adjacent(previous: dict[str, Any], current: dict[str, Any]) -> list[dict[str, Any]]:
    checks = [
        ("min_observed_recall_q16", "recall"),
        ("mean_recall_q16", "recall"),
        ("graph_nodes", "graph_shape"),
        ("graph_edges", "graph_shape"),
        ("upper_layers", "graph_shape"),
        ("upper_graph_edges", "graph_shape"),
    ]
    regressions: list[dict[str, Any]] = []
    for field, kind in checks:
        delta = int(current[field]) - int(previous[field])
        if delta < 0:
            regressions.append(regression(kind, field, previous, current, delta))
    for field in ["hnsw_max_neighbors", "hnsw_ef_search", "hnsw_layer_count"]:
        delta = int(current[field]) - int(previous[field])
        if delta != 0: regressions.append(regression("hnsw_config", field, previous, current, delta))
    for field in ["p95_latency_nanos", "max_latency_nanos"]:
        delta = int(current[field]) - int(previous[field])
        if delta > 0: regressions.append(regression("latency", field, previous, current, delta))
    if previous["passed"] and not current["passed"]: regressions.append(regression("gate", "passed", previous, current, -1))
    if previous["production_safe"] and not current["production_safe"]: regressions.append(regression("gate", "production_safe", previous, current, -1))
    return regressions


def regression(kind: str, field: str, previous: dict[str, Any], current: dict[str, Any], delta: int) -> dict[str, Any]:
    return {
        "kind": kind,
        "field": field,
        "previous_run_id": previous["run_id"],
        "current_run_id": current["run_id"],
        "delta": delta,
    }


def summarize_history(run_root: Path) -> dict[str, Any]:
    manifests = sorted(run_root.glob("*/manifest.json"))
    runs = [load_run(path) for path in manifests]
    runs.sort(key=lambda run: (run["corpus_key"], run["run_id"]))
    groups: dict[str, list[dict[str, Any]]] = {}
    for run in runs:
        groups.setdefault(run["corpus_key"], []).append(run)
    corpora: list[dict[str, Any]] = []
    all_regressions: list[dict[str, Any]] = []
    for key, group_runs in sorted(groups.items()):
        regressions: list[dict[str, Any]] = []
        for previous, current in zip(group_runs, group_runs[1:]):
            regressions.extend(compare_adjacent(previous, current))
        all_regressions.extend(regressions)
        latest = group_runs[-1]
        corpora.append({
            "corpus_key": key,
            "run_count": len(group_runs),
            "latest_run_id": latest["run_id"],
            "latest_git_sha": latest["git_sha"],
            "metric": latest["metric"],
            "vector_count": latest["vector_count"],
            "query_count": latest["query_count"],
            "dimension": latest["dimension"],
            "hnsw_max_neighbors": latest["hnsw_max_neighbors"],
            "hnsw_ef_search": latest["hnsw_ef_search"],
            "hnsw_layer_count": latest["hnsw_layer_count"],
            "latest_min_observed_recall_q16": latest["min_observed_recall_q16"],
            "latest_mean_recall_q16": latest["mean_recall_q16"],
            "latest_p95_latency_nanos": latest["p95_latency_nanos"],
            "latest_production_safe": latest["production_safe"],
            "regressions": regressions,
        })
    latest_run_id = runs[-1]["run_id"] if runs else ""
    return {
        "run_root": str(run_root),
        "run_count": len(runs),
        "corpus_count": len(corpora),
        "latest_run_id": latest_run_id,
        "regression_count": len(all_regressions),
        "regressions": all_regressions,
        "corpora": corpora,
        "runs": runs,
    }


def write_summary(summary: dict[str, Any], output: Path | None) -> None:
    text = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if output is None:
        sys.stdout.write(text)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-root", type=Path, default=Path("target/ann/corpus-runs"))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--fail-on-regression", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    summary = summarize_history(args.run_root)
    write_summary(summary, args.output)
    if args.fail_on_regression and summary["regression_count"] > 0:
        return 1
    return 0


class SelfTests(unittest.TestCase):
    def write_run(self, root: Path, run_id: str, recall: int, p95: int, safe: bool) -> None:
        run_dir = root / run_id
        run_dir.mkdir(parents=True)
        manifest = {
            "run_id": run_id,
            "git_sha": f"sha-{run_id}",
            "metric": "dot_product",
            "vectors": "vectors.jsonl",
            "queries": "queries.jsonl",
            "ground_truth": "ground_truth.jsonl",
            "baseline_report": "",
            "report": str(run_dir / "report.json"),
        }
        report = {
            "passed": safe,
            "failures": [] if safe else ["unsafe"],
            "metric": "dot_product",
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
            "production_safe": safe,
            "queries": [],
        }
        (run_dir / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (run_dir / "report.json").write_text(json.dumps(report), encoding="utf-8")

    def test_history_summary_detects_adjacent_regressions(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            self.write_run(root, "20260101T000000Z-a", 65535, 10, True)
            self.write_run(root, "20260102T000000Z-b", 49151, 20, False)
            summary = summarize_history(root)
        fields = {regression["field"] for regression in summary["regressions"]}
        self.assertEqual(summary["run_count"], 2)
        self.assertEqual(summary["corpus_count"], 1)
        self.assertIn("min_observed_recall_q16", fields)
        self.assertIn("p95_latency_nanos", fields)
        self.assertIn("production_safe", fields)

    def test_empty_history_is_valid(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            summary = summarize_history(Path(raw_dir))
        self.assertEqual(summary["run_count"], 0)
        self.assertEqual(summary["regression_count"], 0)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
