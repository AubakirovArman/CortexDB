#!/usr/bin/env python3
"""Render ANN/HNSW workload SLO profiles as command-line arguments."""

from __future__ import annotations

import argparse
import json
import shlex
import unittest
from dataclasses import asdict, dataclass


@dataclass(frozen=True)
class SloProfile:
    name: str
    min_recall_q16: int
    min_mean_recall_q16: int
    max_p95_latency_nanos: int
    max_p99_latency_nanos: int
    max_max_latency_nanos: int
    max_neighbors: int
    ef_search: int
    ef_construction: int
    layer_count: int


PROFILES = {
    "fast": SloProfile(
        name="fast",
        min_recall_q16=49_151,
        min_mean_recall_q16=49_151,
        max_p95_latency_nanos=50_000_000,
        max_p99_latency_nanos=80_000_000,
        max_max_latency_nanos=100_000_000,
        max_neighbors=8,
        ef_search=64,
        ef_construction=64,
        layer_count=3,
    ),
    "balanced": SloProfile(
        name="balanced",
        min_recall_q16=49_151,
        min_mean_recall_q16=49_151,
        max_p95_latency_nanos=100_000_000,
        max_p99_latency_nanos=200_000_000,
        max_max_latency_nanos=250_000_000,
        max_neighbors=16,
        ef_search=128,
        ef_construction=128,
        layer_count=4,
    ),
    "semantic": SloProfile(
        name="semantic",
        min_recall_q16=60_000,
        min_mean_recall_q16=62_000,
        max_p95_latency_nanos=250_000_000,
        max_p99_latency_nanos=400_000_000,
        max_max_latency_nanos=500_000_000,
        max_neighbors=24,
        ef_search=192,
        ef_construction=256,
        layer_count=5,
    ),
    "audit": SloProfile(
        name="audit",
        min_recall_q16=65_535,
        min_mean_recall_q16=65_535,
        max_p95_latency_nanos=1_000_000_000,
        max_p99_latency_nanos=1_500_000_000,
        max_max_latency_nanos=2_000_000_000,
        max_neighbors=32,
        ef_search=256,
        ef_construction=384,
        layer_count=5,
    ),
}


def profile_for(name: str) -> SloProfile:
    try:
        return PROFILES[name]
    except KeyError as error:
        allowed = ", ".join(sorted(PROFILES))
        raise ValueError(f"unknown SLO profile '{name}', expected one of: {allowed}") from error


def run_external_args(profile: SloProfile) -> list[str]:
    return [
        "--min-recall-q16",
        str(profile.min_recall_q16),
        "--min-mean-recall-q16",
        str(profile.min_mean_recall_q16),
        "--max-p95-latency-nanos",
        str(profile.max_p95_latency_nanos),
        "--max-p99-latency-nanos",
        str(profile.max_p99_latency_nanos),
        "--max-max-latency-nanos",
        str(profile.max_max_latency_nanos),
        "--max-neighbors",
        str(profile.max_neighbors),
        "--ef-search",
        str(profile.ef_search),
        "--ef-construction",
        str(profile.ef_construction),
        "--layer-count",
        str(profile.layer_count),
    ]


def render(profile: SloProfile, output_format: str) -> str:
    if output_format == "json":
        return json.dumps(asdict(profile), sort_keys=True, separators=(",", ":"))
    if output_format == "run-external-args":
        return " ".join(shlex.quote(value) for value in run_external_args(profile))
    raise ValueError(f"unsupported output format: {output_format}")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=sorted(PROFILES), default="balanced")
    parser.add_argument("--format", choices=["json", "run-external-args"], default="json")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    print(render(profile_for(args.profile), args.format))
    return 0


class SelfTests(unittest.TestCase):
    def test_balanced_matches_existing_defaults(self) -> None:
        profile = profile_for("balanced")
        self.assertEqual(profile.min_recall_q16, 49_151)
        self.assertEqual(profile.max_p95_latency_nanos, 100_000_000)
        self.assertEqual(profile.max_p99_latency_nanos, 200_000_000)
        self.assertEqual(profile.max_neighbors, 16)
        self.assertEqual(profile.ef_search, 128)
        self.assertEqual(profile.ef_construction, 128)
        self.assertEqual(profile.layer_count, 4)

    def test_audit_is_high_recall(self) -> None:
        profile = profile_for("audit")
        self.assertEqual(profile.min_recall_q16, 65_535)
        self.assertEqual(profile.min_mean_recall_q16, 65_535)
        self.assertGreater(profile.max_neighbors, profile_for("balanced").max_neighbors)

    def test_run_external_args_include_thresholds_and_graph_knobs(self) -> None:
        rendered = render(profile_for("semantic"), "run-external-args")
        self.assertIn("--min-recall-q16 60000", rendered)
        self.assertIn("--max-p99-latency-nanos 400000000", rendered)
        self.assertIn("--max-neighbors 24", rendered)
        self.assertIn("--ef-search 192", rendered)
        self.assertIn("--ef-construction 256", rendered)


if __name__ == "__main__":
    try:
        raise SystemExit(main(__import__("sys").argv[1:]))
    except ValueError as error:
        print(f"error: {error}", file=__import__("sys").stderr)
        raise SystemExit(2)
