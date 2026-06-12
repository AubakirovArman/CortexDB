#!/usr/bin/env python3
"""Validate the community roadmap board contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "docs/COMMUNITY_ROADMAP.md": [
        "## Roadmap Contract",
        "Add milestones",
        "Add beta blockers",
        "Add production blockers",
        "Add experimental tracks",
        "## Milestones",
        "Beta contract freeze",
        "Production v1 local boundary",
        "ANN guarded promotion",
        "Dashboard product surface",
        "Community contribution loop",
        "## Beta Blockers",
        "Repeatable real-domain ANN history",
        "SDK publication discipline",
        "Product UI beta readiness",
        "Consensus beta evidence",
        "## Production Blockers",
        "Distributed consensus is not production HA",
        "Managed cloud is not implemented",
        "Enterprise RBAC/compliance is future work",
        "Legal-grade verification is out of scope",
        "## Experimental Tracks",
        "Production distributed consensus",
        "Managed cloud",
        "Enterprise RBAC/compliance",
        "Full HNSW without fallback",
        "Built-in LLM inference",
        "External identity",
        "Legal-grade verification",
        "make community-roadmap-check",
        "public product claim",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "COMMUNITY_ROADMAP.md",
        "community roadmap",
    ],
    "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md": [
        "### Epic 140. Community Roadmap Board",
        "Status: done",
        "docs/COMMUNITY_ROADMAP.md",
        "scripts/community_roadmap_check.py",
        "make community-roadmap-check",
    ],
    "Makefile": [
        "COMMUNITY_ROADMAP_REPORT",
        "community-roadmap-check",
        "scripts/community_roadmap_check.py",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/community-roadmap/report.json")
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
        "schema_version": "cortexdb.community_roadmap.report.v1",
        "status": "failed" if failures else "passed",
        "files_checked": sorted(REQUIRED_MARKERS),
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"community roadmap check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"community roadmap check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
