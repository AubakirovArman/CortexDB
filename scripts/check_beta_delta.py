#!/usr/bin/env python3
"""Validate the beta delta note and its release-gate wiring."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


REQUIRED_DOC_TERMS = (
    "Core Alpha",
    "v0.2.0-beta.1",
    "Stable Now",
    "Experimental Or Guarded",
    "Blocked Before Beta Promotion",
    "BETA_RELEASE.md",
    "make production-evidence-sweep",
    "make beta-foundation-check",
    "make beta-rc-check",
    "make production-hardening-check",
    "make production-candidate-check",
    "make production-v1-check",
    "make ann-real-embedding-readiness",
    "make beta-delta-check",
    "make beta-release-check",
    "CORTEXDB_EMBEDDING_URL",
    "CORTEXDB_EMBEDDING_MODEL",
    "production_safe",
    "SDK publication",
)

REQUIRED_MAKEFILE_TERMS = (
    "beta-delta-check:",
    "beta-foundation-check:",
    "beta-rc-check:",
    "beta-release-check:",
    "production-hardening-check:",
    "production-candidate-check:",
    "production-v1-check:",
    "python3 scripts/check_beta_delta.py",
    "python3 scripts/beta_foundation_check.py",
    "python3 scripts/beta_rc_check.py",
    "python3 scripts/beta_release_bundle.py",
    "python3 scripts/production_hardening_check.py",
    "python3 scripts/production_candidate_check.py",
    "python3 scripts/production_v1_check.py",
    "$(MAKE) beta-delta-check",
)

REQUIRED_PLAN_TERMS = (
    "BETA_DELTA.md",
    "BETA_RELEASE.md",
    "make beta-delta-check",
    "make beta-release-check",
)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def missing_terms(label: str, text: str, terms: tuple[str, ...]) -> list[str]:
    return [f"{label}: missing {term!r}" for term in terms if term not in text]


def validate(repo: Path) -> list[str]:
    errors: list[str] = []
    beta_delta = read(repo / "docs/archive/BETA_DELTA.md")
    makefile = read(repo / "Makefile")
    plan = read(repo / "docs/archive/REMAINING_EXECUTION_PLAN.md")

    errors.extend(missing_terms("docs/archive/BETA_DELTA.md", beta_delta, REQUIRED_DOC_TERMS))
    errors.extend(missing_terms("Makefile", makefile, REQUIRED_MAKEFILE_TERMS))
    errors.extend(missing_terms("docs/archive/REMAINING_EXECUTION_PLAN.md", plan, REQUIRED_PLAN_TERMS))
    return errors


def self_test() -> int:
    fake_repo = Path("/tmp/nonexistent-cortexdb-beta-delta-self-test")
    try:
        validate(fake_repo)
    except AssertionError as exc:
        if "missing file" in str(exc):
            print("beta delta self-test passed")
            return 0
    print("beta delta self-test failed")
    return 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    repo = Path(__file__).resolve().parent.parent
    try:
        errors = validate(repo)
    except AssertionError as exc:
        errors = [str(exc)]

    if errors:
        print("BETA DELTA CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1

    print("beta delta check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
