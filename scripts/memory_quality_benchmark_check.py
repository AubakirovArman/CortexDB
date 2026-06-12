#!/usr/bin/env python3
"""Validate and run the deterministic memory quality benchmark."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


REQUIRED_MARKERS = {
    "docs/MEMORY_QUALITY_BENCHMARK.md": [
        "## Benchmark Contract",
        "Benchmark update handling",
        "Benchmark stale memory detection",
        "Benchmark preference retrieval",
        "Benchmark temporal changes",
        "make memory-quality-benchmark-check",
    ],
    "crates/cortex-engine/tests/memory_quality_benchmark.rs": [
        "memory_quality_update_handling_prefers_latest_payload",
        "memory_quality_stale_memory_detection_expires_and_scores_memory",
        "memory_quality_preference_retrieval_uses_feedback_signal",
        "memory_quality_temporal_changes_preserve_snapshot_visibility",
    ],
    "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md": [
        "### Epic 142. Memory Quality Benchmark",
        "Status: done",
        "docs/MEMORY_QUALITY_BENCHMARK.md",
        "make memory-quality-benchmark-check",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "MEMORY_QUALITY_BENCHMARK.md",
    ],
    "Makefile": [
        "MEMORY_QUALITY_BENCHMARK_REPORT",
        "memory-quality-benchmark-check",
        "scripts/memory_quality_benchmark_check.py",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/memory-quality-benchmark/report.json")
    return parser.parse_args()


def run(command: list[str]) -> None:
    subprocess.run(command, check=True)


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    for file_name, markers in REQUIRED_MARKERS.items():
        path = Path(file_name)
        if not path.is_file():
            failures.append(f"missing {file_name}")
            continue
        text = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in text:
                failures.append(f"{file_name}: missing {marker!r}")

    test_command = [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "memory_quality_benchmark",
    ]
    if not failures:
        run(test_command)

    report = {
        "schema_version": "cortexdb.memory_quality_benchmark.report.v1",
        "status": "failed" if failures else "passed",
        "test_command": " ".join(test_command),
        "files_checked": sorted(REQUIRED_MARKERS),
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"memory quality benchmark check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"memory quality benchmark check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
