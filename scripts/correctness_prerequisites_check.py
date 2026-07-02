#!/usr/bin/env python3
"""Validate the aggregate correctness-prerequisites release gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_SUBGATES = [
    "$(MAKE) cosine-metric-correctness-check",
    "$(MAKE) cell-id-collision-check",
    "$(MAKE) conflict-normalization-check",
    "$(MAKE) ann-budget-disclosure-check",
    "$(MAKE) ann-metric-matrix-check",
    "$(MAKE) context-pack-conflict-visibility-check",
    "$(MAKE) engine-determinism-check",
]

REQUIRED_MAKE_TERMS = [
    "CORRECTNESS_PREREQUISITES_REPORT ?= target/correctness-prerequisites/report.json",
    "correctness-prerequisites-check:",
    "python3 scripts/correctness_prerequisites_check.py --root \".\" --report \"$(CORRECTNESS_PREREQUISITES_REPORT)\"",
]

REQUIRED_PHONY_TERMS = [
    "correctness-prerequisites-check",
]

REQUIRED_RELEASE_TERMS = [
    "$(MAKE) correctness-prerequisites-check",
]

REQUIRED_BETA_BUNDLE_TERMS = [
    '"name": "correctness_prerequisites"',
    '"command": ["make", "correctness-prerequisites-check"]',
    '"target/correctness-prerequisites/report.json"',
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")
    release = read_text(root / "mk/release.mk")
    beta_bundle = read_text(root / "scripts/beta_release_bundle.py")

    failures: list[str] = []
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("correctness-prerequisites-check", makefiles, REQUIRED_SUBGATES))
    failures.extend(missing_terms("mk/phony.mk", phony, REQUIRED_PHONY_TERMS))
    failures.extend(missing_terms("mk/release.mk", release, REQUIRED_RELEASE_TERMS))
    failures.extend(
        missing_terms("scripts/beta_release_bundle.py", beta_bundle, REQUIRED_BETA_BUNDLE_TERMS)
    )

    return {
        "schema_version": "cortexdb.correctness_prerequisites.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "subgates": REQUIRED_SUBGATES,
            "make_terms": REQUIRED_MAKE_TERMS,
            "phony_terms": REQUIRED_PHONY_TERMS,
            "release_terms": REQUIRED_RELEASE_TERMS,
            "beta_bundle_terms": REQUIRED_BETA_BUNDLE_TERMS,
        },
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"correctness prerequisites check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
