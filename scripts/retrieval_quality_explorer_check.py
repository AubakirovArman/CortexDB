#!/usr/bin/env python3
"""Validate Retrieval Quality Explorer evidence wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "dashboard_renderer": [
        ("scripts/retrieval_quality_dashboard.py", "CortexDB Retrieval Quality Dashboard"),
        ("scripts/retrieval_quality_dashboard.py", "Recall"),
        ("scripts/retrieval_quality_dashboard.py", "MRR"),
        ("scripts/retrieval_quality_dashboard.py", "nDCG"),
        ("scripts/retrieval_quality_dashboard.py", "p95 latency"),
        ("scripts/retrieval_quality_dashboard.py", "Domain Quality Table"),
        ("scripts/retrieval_quality_dashboard.py", "Query-Level Table"),
        ("scripts/retrieval_quality_dashboard.py", "latest_mean_recall_q16"),
        ("scripts/retrieval_quality_dashboard.py", "latest_mean_mrr_q16"),
        ("scripts/retrieval_quality_dashboard.py", "latest_mean_ndcg_q16"),
        ("scripts/retrieval_quality_dashboard.py", "latest_p95_latency_nanos"),
        ("scripts/retrieval_quality_dashboard.py", "query_level"),
    ],
    "dashboard_panels": [
        ("scripts/retrieval_quality_dashboard_panels.py", "Recall Panel"),
        ("scripts/retrieval_quality_dashboard_panels.py", "MRR Panel"),
        ("scripts/retrieval_quality_dashboard_panels.py", "nDCG Panel"),
        ("scripts/retrieval_quality_dashboard_panels.py", "Latency Trend Panel"),
    ],
    "docs": [
        ("docs/archive/DASHBOARD_UI.md", "Retrieval Quality Explorer"),
        ("docs/RETRIEVAL_QUALITY_EVIDENCE.md", "target/retrieval-quality/dashboard.html"),
        ("docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Epic 114. Retrieval Quality Explorer"),
    ],
    "make": [
        ("Makefile", "retrieval-quality-check"),
        ("Makefile", "retrieval_quality_dashboard.py"),
        ("Makefile", "RETRIEVAL_QUALITY_DASHBOARD"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except RuntimeError as error:
        print(f"retrieval quality explorer check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"retrieval quality explorer check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
