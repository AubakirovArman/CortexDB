#!/usr/bin/env python3
"""Validate neutral CortexDB comparison documentation."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "docs/COMPARISONS.md": [
        "## Epic 138 Comparison Docs v2 Contract",
        "Compare with vector DB",
        "Compare with RAG storage",
        "Compare with Postgres",
        "Compare with memory frameworks",
        "Compare with document search",
        "PostgreSQL / SQL databases",
        "Vector databases",
        "Classic RAG stacks",
        "Agent memory frameworks",
        "Document search engines",
        "ContextPack output",
        "VERIFY FACT",
        "does not claim",
        "full PostgreSQL/SQLite replacement",
        "managed vector database replacement",
        "production distributed consensus",
        "fallback-free production HNSW",
        "PUBLIC_CLAIMS_POLICY.md",
    ],
    "README.md": [
        "docs/COMPARISONS.md",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "COMPARISONS.md",
    ],
    "docs/RAG_VS_CORTEXDB.md": [
        "load-fixture ./demo-db examples/datasets/investment_projects",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/comparison-docs/report.json")
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
        "schema_version": "cortexdb.comparison_docs.report.v2",
        "status": "failed" if failures else "passed",
        "files_checked": sorted(REQUIRED_MARKERS),
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"comparison docs check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"comparison docs check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
