#!/usr/bin/env python3
"""Validate the package-manager feasibility decision."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "docs/PACKAGE_MANAGER_FEASIBILITY.md": [
        "Homebrew Formula Evaluation",
        "Linux Package Evaluation",
        "Decision",
        "not claim that CortexDB is already published",
        "project-owned tap",
        "Debian control metadata",
        "RPM spec metadata",
        "package install smoke tests",
        "make binary-release-check",
        "docs/SYSTEMD.md",
        "docs/LAUNCHD.md",
        "Homebrew Formula Cookbook",
        "Debian binary package policy",
    ],
    "docs/BINARY_RELEASES.md": [
        "PACKAGE_MANAGER_FEASIBILITY.md",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "PACKAGE_MANAGER_FEASIBILITY.md",
    ],
    "docs/PRODUCTION_EPIC_EXECUTION_PLAN.md": [
        "Epic 130, Homebrew/Package Manager Feasibility",
        "make package-manager-feasibility-check",
        "Homebrew tap is feasible",
        "Linux `.deb` and `.rpm` packaging is feasible",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/package-manager-feasibility/report.json")
    return parser.parse_args()


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parent.parent
    failures: list[str] = []
    checked: list[str] = []

    for relative, markers in REQUIRED_MARKERS.items():
        checked.append(relative)
        path = repo / relative
        if not path.is_file():
            failures.append(f"missing {relative}")
            continue
        text = read(path)
        for marker in markers:
            if marker not in text:
                failures.append(f"{relative}: missing {marker!r}")

    report = {
        "schema_version": "cortexdb.package_manager_feasibility.v1",
        "status": "failed" if failures else "passed",
        "checked": checked,
        "decision": {
            "homebrew": "feasible_after_formula_template",
            "linux": "feasible_after_deb_rpm_templates",
            "published": False,
        },
        "failures": failures,
    }
    output = repo / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"package manager feasibility check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"package manager feasibility check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
