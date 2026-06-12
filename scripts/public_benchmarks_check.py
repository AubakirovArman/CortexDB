#!/usr/bin/env python3
"""Validate the public benchmark history page."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "docs/PUBLIC_BENCHMARKS.md": [
        "v0.1.0-core-alpha.5",
        "v0.2.0-beta.1",
        "## Epic 136 Task Coverage",
        "Storage benchmarks",
        "Retrieval benchmarks",
        "ContextPack benchmarks",
        "Verify benchmarks",
        "LongMemEval results",
        "Release trends",
        "make beta-release-check",
        "make public-retrieval-benchmark-page-check",
        "PUBLIC_RETRIEVAL_BENCHMARKS.md",
        "make retrieval-quality-check",
        "make context-pack-quality-check",
        "make verification-quality-check",
        "make single-node-performance-check",
        "make longmemeval-v1-official-retrieval-metrics",
        "make performance-trend-check",
        "production_safe=true",
        "mean_recall_q16=65535",
        "evidence_coverage_q16=65535",
        "case_count: 203",
        "session recall_all@10=0.9021",
        "Release trend comparison",
        "PUBLIC_CLAIMS_POLICY.md",
        "production distributed consensus",
        "fallback-free production HNSW",
    ],
    "README.md": [
        "docs/PUBLIC_BENCHMARKS.md",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "PUBLIC_BENCHMARKS.md",
        "PUBLIC_RETRIEVAL_BENCHMARKS.md",
    ],
    "docs/PUBLIC_RETRIEVAL_BENCHMARKS.md": [
        "## Dataset Size",
        "## Latest Local Metrics",
        "## Exact Vs ANN",
        "## Limitations",
        "investment_projects",
        "legal_policies",
        "support_tickets",
        "technical_docs",
        "p95/p99/max",
        "fallback-free production HNSW",
    ],
    "docs/RETRIEVAL_QUALITY_EVIDENCE.md": [
        "production_safe: true",
        "mean_recall_q16: 65535",
    ],
    "docs/archive/CONTEXT_PACK_QUALITY_EVIDENCE.md": [
        "evidence_coverage_q16: 65535",
        "domain_count: 5",
    ],
    "docs/VERIFICATION_QUALITY_EVIDENCE.md": [
        "case_count",
        "domain_counts",
    ],
    "docs/BENCHMARKS.md": [
        "single-node-performance-check",
        "LongMemEval Official Evidence",
    ],
    "docs/LONGMEMEVAL_OFFICIAL.md": [
        "official LongMemEval v1",
        "not an official published LongMemEval leaderboard entry",
    ],
    "docs/PERFORMANCE_TREND_HISTORY.md": [
        "p50/p95/p99",
        "performance-trend-check",
    ],
}

TASK_MARKERS = {
    "storage_benchmarks": ["Storage benchmarks", "make single-node-performance-check"],
    "retrieval_benchmarks": ["Retrieval benchmarks", "make public-retrieval-benchmark-page-check"],
    "contextpack_benchmarks": ["ContextPack benchmarks", "make context-pack-quality-check"],
    "verify_benchmarks": ["Verify benchmarks", "make verification-quality-check"],
    "longmemeval_results": ["LongMemEval results", "session recall_all@10=0.9021"],
    "release_trends": ["Release trends", "make performance-trend-check"],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/public-benchmarks/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    page_text = ""
    for file_name, markers in REQUIRED_MARKERS.items():
        path = Path(file_name)
        if not path.is_file():
            failures.append(f"missing {file_name}")
            continue
        text = path.read_text(encoding="utf-8")
        if file_name == "docs/PUBLIC_BENCHMARKS.md":
            page_text = text
        for marker in markers:
            if marker not in text:
                failures.append(f"{file_name}: missing {marker!r}")

    task_coverage = {}
    for task, markers in TASK_MARKERS.items():
        task_coverage[task] = all(marker in page_text for marker in markers)
        if not task_coverage[task]:
            failures.append(f"docs/PUBLIC_BENCHMARKS.md: Epic 136 task not covered: {task}")

    report = {
        "schema_version": "cortexdb.public_benchmarks.report.v1",
        "status": "failed" if failures else "passed",
        "files_checked": sorted(REQUIRED_MARKERS),
        "task_coverage": task_coverage,
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"public benchmarks check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"public benchmarks check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
