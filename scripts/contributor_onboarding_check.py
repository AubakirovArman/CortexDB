#!/usr/bin/env python3
"""Validate contributor onboarding docs and starter issue map."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "CONTRIBUTING.md": [
        "docs/CONTRIBUTOR_ONBOARDING.md",
        "make contributor-onboarding-check",
        "docs/GOOD_FIRST_ISSUES.md",
    ],
    "docs/CONTRIBUTOR_ONBOARDING.md": [
        "15-minute Path",
        "make contributor-onboarding-check",
        "make use-case-pack-check",
        "docs/MODULE_OWNERSHIP.md",
        "docs/GOOD_FIRST_ISSUES.md",
        "cargo test --workspace --all-features",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "make openapi-contract-check",
        "cargo test -p cortex-cli",
        "docs/PUBLIC_CLAIMS_POLICY.md",
    ],
    "docs/GOOD_FIRST_ISSUES.md": [
        "AQL diagnostics",
        "ContextPack docs",
        "Verification fixtures",
        "Use-case packs",
        "CLI docs",
        "API docs",
        "make use-case-pack-check",
        "make openapi-contract-check",
        "good first issue",
    ],
    ".github/ISSUE_TEMPLATE/good_first_issue.md": [
        "Good first issue",
        "make contributor-onboarding-check",
        "cargo fmt --check",
        "docs/PUBLIC_CLAIMS_POLICY.md",
    ],
    "docs/DOCUMENTATION_INDEX.md": [
        "CONTRIBUTOR_ONBOARDING.md",
        "GOOD_FIRST_ISSUES.md",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/contributor-onboarding/report.json")
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
        "schema_version": "cortexdb.contributor_onboarding.report.v1",
        "status": "failed" if failures else "passed",
        "files_checked": sorted(REQUIRED_MARKERS),
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"contributor onboarding check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"contributor onboarding check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
