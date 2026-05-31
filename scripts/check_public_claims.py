#!/usr/bin/env python3
"""Validate public-facing CortexDB product claims."""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path


PUBLIC_DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "README.md": (
        "experimental Core Alpha",
        "not recommended for production workloads",
        "Long-Term Vision",
    ),
    "docs/API.md": (
        "Core Alpha",
        "not a production SLA",
        "OpenAPI contract",
    ),
    "docs/ARCHITECTURE.md": (
        "experimental Core Alpha",
        "not a production distributed database",
        "future product layers",
    ),
    "docs/PROJECT_STATUS.md": (
        "Core Alpha",
        "not ready for critical high-availability production databases",
        "Experimental",
    ),
    "docs/BETA_DELTA.md": (
        "Stable Now",
        "Experimental Or Guarded",
        "Blocked Before Beta Promotion",
    ),
    "docs/PUBLIC_CLAIMS_POLICY.md": (
        "make public-claims-check",
        "Disallowed Claims",
        "Required Qualifiers",
    ),
}

FORBIDDEN_PHRASES = (
    "ultra-high-performance",
    "Fully Completed & Stable",
    "fully production-grade",
    "enterprise-ready",
    "production workloads supported",
    "production workloads ready",
    "production-ready database",
    "SLA-backed",
)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def missing_terms(label: str, text: str, terms: tuple[str, ...]) -> list[str]:
    return [f"{label}: missing required qualifier {term!r}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str) -> list[str]:
    lowered = text.lower()
    return [
        f"{label}: forbidden public overclaim {term!r}"
        for term in FORBIDDEN_PHRASES
        if term.lower() in lowered
    ]


def validate(repo: Path) -> list[str]:
    errors: list[str] = []
    for relative, terms in PUBLIC_DOC_REQUIREMENTS.items():
        text = read(repo / relative)
        errors.extend(missing_terms(relative, text, terms))
        if relative != "docs/PUBLIC_CLAIMS_POLICY.md":
            errors.extend(forbidden_terms(relative, text))

    makefile = read(repo / "Makefile")
    if "public-claims-check:" not in makefile:
        errors.append("Makefile: missing public-claims-check target")
    if "$(MAKE) public-claims-check" not in makefile:
        errors.append("Makefile: alpha/release gates must run public-claims-check")
    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        for relative, terms in PUBLIC_DOC_REQUIREMENTS.items():
            path = repo / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("\n".join(terms), encoding="utf-8")
        (repo / "Makefile").write_text(
            "public-claims-check:\n\tpython3 scripts/check_public_claims.py\n"
            "alpha-check:\n\t$(MAKE) public-claims-check\n",
            encoding="utf-8",
        )
        clean_errors = validate(repo)
        if clean_errors:
            print("public claims self-test failed on clean fixture")
            for error in clean_errors:
                print(f"  {error}")
            return 1
        (repo / "README.md").write_text(
            "ultra-high-performance production-ready database",
            encoding="utf-8",
        )
        dirty_errors = validate(repo)
        if not dirty_errors:
            print("public claims self-test failed to catch overclaim")
            return 1
    print("public claims self-test passed")
    return 0


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
        print("PUBLIC CLAIMS CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1
    print("public claims check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
