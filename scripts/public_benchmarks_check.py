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
        "make beta-release-check",
        "make retrieval-quality-check",
        "make context-pack-quality-check",
        "make verification-quality-check",
        "make single-node-performance-check",
        "make performance-trend-check",
        "production_safe=true",
        "mean_recall_q16=65535",
        "evidence_coverage_q16=65535",
        "PUBLIC_CLAIMS_POLICY.md",
        "production distributed consensus",
        "fallback-free production HNSW",
    ],
    "README.md": [
        "docs/PUBLIC_BENCHMARKS.md",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "PUBLIC_BENCHMARKS.md",
    ],
    "docs/RETRIEVAL_QUALITY_EVIDENCE.md": [
        "production_safe: true",
        "mean_recall_q16: 65535",
    ],
    "docs/CONTEXT_PACK_QUALITY_EVIDENCE.md": [
        "evidence_coverage_q16: 65535",
        "domain_count: 5",
    ],
    "docs/VERIFICATION_QUALITY_EVIDENCE.md": [
        "case_count",
        "domain_counts",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/public-benchmarks/report.json")
    return parser.parse_args()


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

    report = {
        "schema_version": "cortexdb.public_benchmarks.report.v1",
        "status": "failed" if failures else "passed",
        "files_checked": sorted(REQUIRED_MARKERS),
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
