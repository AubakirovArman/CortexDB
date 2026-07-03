#!/usr/bin/env python3
"""F3.4 (QA half, evidence): validate a scored LoCoMo QA artifact + emit a snapshot.

Turns the official per-category F1 scoring of a LoCoMo QA run (produced by the
metered reader + snap-research/locomo evaluator) into a `benchmark_report.v1`
snapshot draft that is honestly marked `leaderboard_comparable: false` ("local
reader, not an official leaderboard entry"). The snapshot is validated against the
frozen `schemas/benchmark_report.v1.schema.json` before it is written.

Manual-evidence lane (mirrors F2.1): when the scored artifact is ABSENT (no reader
key / no metered run yet), the gate prints a blocked notice and exits 0 — it never
fabricates numbers. It only produces a snapshot when a real scored artifact exists.

Scored-artifact contract (input): a JSON object with `total_questions`,
`completed_questions`, `overall_f1` (0..1), optional `reader_model` /
`judge_protocol`, and `per_category: {name: {count, f1}}`.

  - FAST (offline, `--self-test`): build a snapshot from the committed sample
    scored artifact, assert it validates against the schema and is
    `leaderboard_comparable: false` + byte-deterministic, and assert the
    blocked-exit-0 path fires for a missing artifact. No network, no wall clock.

  - REAL: `--scored <path>` `--output <snapshot.json>` over the metered run's
    scored artifact.

Dependency-free (stdlib only); deterministic.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
import sys
from typing import Any

REPO = pathlib.Path(__file__).resolve().parents[2]
FIX = REPO / "fixtures" / "benchmarks" / "locomo"
SAMPLE_SCORED = FIX / "qa_scored_sample.json"
SCHEMA_PATH = REPO / "schemas" / "benchmark_report.v1.schema.json"
BOUNDARY = "local reader, not an official leaderboard entry"


def _load_validate():
    spec = importlib.util.spec_from_file_location(
        "benchmark_report_schema_check",
        REPO / "scripts" / "benchmark_report_schema_check.py",
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.validate


def build_snapshot(scored: dict[str, Any]) -> dict[str, Any]:
    """Deterministically map a scored QA artifact to a benchmark_report.v1 draft."""
    per_category = scored.get("per_category", {})
    return {
        "aggregate_stats": {
            "total_questions": int(scored["total_questions"]),
            "completed_questions": int(scored["completed_questions"]),
            "average_recall_pct": round(float(scored["overall_f1"]) * 100.0, 4),
        },
        "question_type_stats": {
            category: {
                "count": int(row["count"]),
                "average_recall_pct": round(float(row["f1"]) * 100.0, 4),
            }
            for category, row in sorted(per_category.items())
        },
        "leaderboard_comparable": False,
        "reader_model": scored.get("reader_model", ""),
        "judge_protocol": scored.get("judge_protocol", ""),
        "boundary": BOUNDARY,
    }


def validate_snapshot(snapshot: dict[str, Any]) -> list[str]:
    validate = _load_validate()
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    errors: list[str] = []
    validate(snapshot, schema, "benchmark_report", errors)
    return errors


def run_real(args: argparse.Namespace) -> int:
    scored_path = pathlib.Path(args.scored)
    if not scored_path.exists():
        print(
            f"blocked: scored LoCoMo QA artifact not found at {scored_path} "
            "(run the metered reader + official per-category F1 scoring first); "
            "exit 0 (manual-evidence lane)."
        )
        return 0
    scored = json.loads(scored_path.read_text(encoding="utf-8"))
    snapshot = build_snapshot(scored)
    errors = validate_snapshot(snapshot)
    if errors:
        print("LoCoMo QA evidence snapshot FAILED schema validation:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    out_path = pathlib.Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(
        json.dumps(snapshot, indent=2, sort_keys=True, ensure_ascii=True) + "\n",
        encoding="utf-8",
    )
    print(
        f"wrote leaderboard_comparable=false snapshot -> {out_path} "
        f"(overall {snapshot['aggregate_stats']['average_recall_pct']}%, "
        f"{len(snapshot['question_type_stats'])} categories)"
    )
    return 0


def run_self_test() -> int:
    failures: list[str] = []
    scored = json.loads(SAMPLE_SCORED.read_text(encoding="utf-8"))

    snapshot_a = build_snapshot(scored)
    snapshot_b = build_snapshot(scored)
    if json.dumps(snapshot_a, sort_keys=True) != json.dumps(snapshot_b, sort_keys=True):
        failures.append("snapshot construction is non-deterministic")

    errors = validate_snapshot(snapshot_a)
    if errors:
        failures.append(f"sample snapshot fails benchmark_report.v1 schema: {errors}")

    if snapshot_a.get("leaderboard_comparable") is not False:
        failures.append("snapshot must be marked leaderboard_comparable=false")
    if snapshot_a.get("boundary") != BOUNDARY:
        failures.append("snapshot missing the honest boundary note")

    # Per-category coverage carried through from the scored artifact.
    expected_categories = set(scored["per_category"])
    got_categories = set(snapshot_a["question_type_stats"])
    if expected_categories != got_categories:
        failures.append(f"category mismatch: {expected_categories} != {got_categories}")

    # A malformed snapshot must be REJECTED (proves the validator isn't vacuous).
    broken = json.loads(json.dumps(snapshot_a))
    del broken["aggregate_stats"]["average_recall_pct"]
    if not validate_snapshot(broken):
        failures.append("validator accepted a snapshot missing average_recall_pct")

    # Blocked path: a missing scored artifact exits 0 without writing anything.
    args = argparse.Namespace(scored=str(FIX / "does_not_exist.json"), output="/dev/null")
    if run_real(args) != 0:
        failures.append("missing scored artifact did not exit 0 (manual-evidence lane)")

    if failures:
        print("F3.4 LoCoMo QA evidence self-test FAILED:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print(
        "F3.4 LoCoMo QA evidence self-test passed: sample snapshot validates against "
        "benchmark_report.v1, is leaderboard_comparable=false + byte-deterministic, "
        "the validator rejects a malformed snapshot, and a missing artifact blocks "
        "(exit 0)."
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="offline harness check")
    parser.add_argument("--scored", help="scored LoCoMo QA artifact JSON (per-category F1)")
    parser.add_argument("--output", help="benchmark_report.v1 snapshot to write")
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()
    if not args.scored or not args.output:
        parser.error("real check needs --scored --output (or pass --self-test)")
    return run_real(args)


if __name__ == "__main__":
    raise SystemExit(main())
