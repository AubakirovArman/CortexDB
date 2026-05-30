#!/usr/bin/env python3
"""Create a release-ready ANN baseline bundle from an archived corpus run."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import tempfile
import unittest
from datetime import datetime, timezone
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


def validate_baseline_id(value: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9._-]+", value):
        raise ValueError("baseline id must use only letters, digits, '.', '_', or '-'")
    return value


def resolve_path(run_dir: Path, raw_value: object) -> Path | None:
    if not isinstance(raw_value, str) or not raw_value:
        return None
    path = Path(raw_value)
    if path.exists():
        return path
    candidate = run_dir / path.name
    return candidate if candidate.exists() else None


def copy_file(src: Path | None, dst: Path, files: dict[str, str]) -> None:
    if src is None or not src.exists():
        return
    shutil.copy2(src, dst)
    files[dst.name] = str(dst)


def run_summary(report: dict[str, Any]) -> dict[str, Any]:
    fields = [
        "passed",
        "metric",
        "hnsw_max_neighbors",
        "hnsw_ef_search",
        "hnsw_layer_count",
        "vector_count",
        "query_count",
        "dimension",
        "graph_nodes",
        "graph_edges",
        "upper_layers",
        "upper_graph_edges",
        "min_observed_recall_q16",
        "mean_recall_q16",
        "p50_latency_nanos",
        "p95_latency_nanos",
        "max_latency_nanos",
        "production_safe",
    ]
    return {field: report.get(field) for field in fields}


def publish_baseline(
    run_root: Path,
    run_id: str,
    output_root: Path,
    baseline_id: str,
    created_at: str,
) -> dict[str, Any]:
    baseline_id = validate_baseline_id(baseline_id)
    run_dir = run_root / run_id
    manifest_path = run_dir / "manifest.json"
    if not manifest_path.exists():
        raise ValueError(f"{manifest_path}: run manifest not found")
    manifest = load_json(manifest_path)
    report_path = resolve_path(run_dir, manifest.get("report")) or (run_dir / "report.json")
    if not report_path.exists():
        raise ValueError(f"{report_path}: report not found")
    report = load_json(report_path)
    output_dir = output_root / baseline_id
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True)
    files: dict[str, str] = {}
    copy_file(manifest_path, output_dir / "run_manifest.json", files)
    copy_file(report_path, output_dir / "report.json", files)
    copy_file(run_root / "history.json", output_dir / "history.json", files)
    copy_file(run_dir / "comparison.json", output_dir / "comparison.json", files)
    copy_file(resolve_path(run_dir, manifest.get("ground_truth")), output_dir / "ground_truth.jsonl", files)
    copy_file(resolve_path(run_dir, manifest.get("machine_profile")), output_dir / "machine_profile.json", files)
    baseline_manifest = {
        "baseline_id": baseline_id,
        "created_at": created_at,
        "source_run_id": run_id,
        "source_run_root": str(run_root),
        "git_sha": manifest.get("git_sha", ""),
        "vectors": manifest.get("vectors", ""),
        "queries": manifest.get("queries", ""),
        "ground_truth": manifest.get("ground_truth", ""),
        "machine_profile": manifest.get("machine_profile", ""),
        "baseline_report": manifest.get("baseline_report", ""),
        "summary": run_summary(report),
        "files": files,
    }
    (output_dir / "baseline_manifest.json").write_text(
        json.dumps(baseline_manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return baseline_manifest


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-root", type=Path, default=Path("target/ann/corpus-runs"))
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--baseline-id")
    parser.add_argument("--output-root", type=Path, default=Path("target/ann/release-baselines"))
    parser.add_argument("--created-at")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    baseline_id = args.baseline_id or args.run_id
    created_at = args.created_at or datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    manifest = publish_baseline(args.run_root, args.run_id, args.output_root, baseline_id, created_at)
    sys.stdout.write(json.dumps(manifest, separators=(",", ":")) + "\n")
    return 0


class SelfTests(unittest.TestCase):
    def write_run(self, root: Path) -> None:
        run_dir = root / "smoke"
        run_dir.mkdir(parents=True)
        manifest = {
            "run_id": "smoke",
            "git_sha": "abc123",
            "metric": "dot_product",
            "vectors": "vectors.jsonl",
            "queries": "queries.jsonl",
            "ground_truth": str(run_dir / "ground_truth.jsonl"),
            "baseline_report": "",
            "machine_profile": str(run_dir / "machine_profile.json"),
            "report": str(run_dir / "report.json"),
        }
        report = {
            "passed": True,
            "metric": "dot_product",
            "vector_count": 2,
            "query_count": 1,
            "dimension": 2,
            "hnsw_max_neighbors": 8,
            "hnsw_ef_search": 64,
            "hnsw_layer_count": 4,
            "graph_nodes": 2,
            "graph_edges": 2,
            "upper_layers": 1,
            "upper_graph_edges": 1,
            "min_observed_recall_q16": 65535,
            "mean_recall_q16": 65535,
            "p50_latency_nanos": 10,
            "p95_latency_nanos": 10,
            "max_latency_nanos": 10,
            "production_safe": True,
            "queries": [],
        }
        (run_dir / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (run_dir / "report.json").write_text(json.dumps(report), encoding="utf-8")
        (run_dir / "machine_profile.json").write_text('{"schema_version":1}\n', encoding="utf-8")
        (run_dir / "ground_truth.jsonl").write_text('{"name":"q","candidates":[1]}\n', encoding="utf-8")
        (root / "history.json").write_text(json.dumps({"run_count": 1}), encoding="utf-8")

    def test_publish_baseline_copies_release_files(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir) / "runs"
            output_root = Path(raw_dir) / "baselines"
            self.write_run(root)
            manifest = publish_baseline(root, "smoke", output_root, "v0.1-smoke", "2026-01-01T00:00:00Z")
            bundle = output_root / "v0.1-smoke"
            self.assertEqual(manifest["git_sha"], "abc123")
            self.assertTrue((bundle / "baseline_manifest.json").exists())
            self.assertTrue((bundle / "run_manifest.json").exists())
            self.assertTrue((bundle / "report.json").exists())
            self.assertTrue((bundle / "history.json").exists())
            self.assertTrue((bundle / "ground_truth.jsonl").exists())
            self.assertTrue((bundle / "machine_profile.json").exists())

    def test_invalid_baseline_id_is_rejected(self) -> None:
        with self.assertRaises(ValueError):
            validate_baseline_id("../bad")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
