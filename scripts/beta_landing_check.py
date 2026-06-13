#!/usr/bin/env python3
"""Validate the concise public beta landing page."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "docs/BETA_LANDING.md": [
        "v0.2.0-beta.2",
        "## One-liner",
        "## Value Proposition",
        "## Quickstart",
        "## Demo",
        "## Architecture Diagram",
        "## Limitations",
        "make beta-release-check",
        "make demo",
        "/v1/compatibility",
        "ContextPack generation",
        "VERIFY FACT",
        "single-node durable storage",
        "production multi-node consensus",
        "managed cloud operations",
        "fallback-free production HNSW",
        "make openapi-contract-check",
        "make sdk-e2e-release-check",
        "make dashboard-product-check",
        "PUBLIC_CLAIMS_POLICY.md",
    ],
    "README.md": [
        "docs/BETA_LANDING.md",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "BETA_LANDING.md",
    ],
}


TASK_COVERAGE = {
    "one_liner": ["## One-liner", "agent-native context database"],
    "demo": ["## Demo", "load-fixture", "search", "context", "verify"],
    "value_proposition": ["## Value Proposition", "permission-aware context"],
    "limitations": ["## Limitations", "does not claim"],
    "quickstart": ["## Quickstart", "git clone", "make beta-release-check"],
    "architecture_diagram": ["## Architecture Diagram", "WAL -> MemTable MVCC"],
}


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/beta-landing/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    landing = ""
    for file_name, markers in REQUIRED_MARKERS.items():
        path = Path(file_name)
        if not path.is_file():
            failures.append(f"missing {file_name}")
            continue
        text = read(path)
        if file_name == "docs/BETA_LANDING.md":
            landing = text
        for marker in markers:
            if marker not in text:
                failures.append(f"{file_name}: missing {marker!r}")

    task_coverage = {
        task: all(marker in landing for marker in markers)
        for task, markers in TASK_COVERAGE.items()
    }
    for task, covered in task_coverage.items():
        if not covered:
            failures.append(f"docs/BETA_LANDING.md: task coverage missing {task}")

    report = {
        "schema_version": 1,
        "status": "failed" if failures else "passed",
        "files_checked": sorted(REQUIRED_MARKERS),
        "task_coverage": task_coverage,
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"beta landing check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"beta landing check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
