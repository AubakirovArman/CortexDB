#!/usr/bin/env python3
"""Validate public-facing CortexDB product claims."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


PUBLIC_DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "README.md": (
        "single-node agent-native database beta",
        "v0.2.0-beta.2",
        "not recommended for production workloads",
        "Honesty Snapshot",
    ),
    "docs/API.md": (
        "v0.2.0-beta.2",
        "not a production SLA",
        "OpenAPI contract",
    ),
    "docs/ARCHITECTURE.md": (
        "single-node agent-native database beta",
        "not a production distributed database",
        "future product layers",
    ),
    "docs/PROJECT_STATUS.md": (
        "single-node agent-native database beta",
        "not a production HA database",
        "Research Prototypes",
        "Frozen Or Not Production",
    ),
    "docs/archive/BETA_DELTA.md": (
        "v0.2.0-beta.2",
        "BETA_RELEASE.md",
        "Stable Now",
        "Experimental Or Guarded",
        "Blocked Before Beta Promotion",
    ),
    "docs/archive/BETA_RELEASE.md": (
        "v0.2.0-beta.2",
        "Core Alpha with Beta Foundation evidence",
        "Explicit Non-Goals For Beta",
        "make beta-release-check",
    ),
    "docs/PUBLIC_CLAIMS_POLICY.md": (
        "single-node agent-native database beta",
        "make public-claims-check",
        "Disallowed Claims",
        "Required Qualifiers",
    ),
    "docs/archive/PUBLIC_CLAIMS_FREEZE.md": (
        "local single-node only",
        "Forbidden Public Claims",
        "Release Gate",
    ),
    "docs/archive/PRODUCTION_V1.md": (
        "local single-node",
        "Distributed Production Is Out Of Scope",
        "not a public SLA",
    ),
    "docs/archive/BINARY_PLATFORM_MATRIX.md": (
        "Windows is unsupported",
        "Clean Install Smoke",
    ),
    "docs/archive/SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md": (
        "Release-blocking rule",
        "Forbidden wording",
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

RISKY_CLAIMS = (
    "production distributed",
    "managed cloud",
    "enterprise compliance",
    "enterprise RBAC",
    "legal-grade",
    "production HNSW",
    "production-ready",
    "tamper-evident audit",
)

SAFE_CONTEXT_MARKERS = (
    "not",
    "no ",
    "do not",
    "out of scope",
    "future",
    "blocked",
    "checklist",
    "design",
    "defer",
    "deferred",
    "experimental",
    "future work",
    "gap",
    "gaps",
    "missing",
    "need",
    "needs",
    "non-goals",
    "not included",
    "not ready",
    "not recommended",
    "next",
    "out of core",
    "p2",
    "unsupported",
    "disallowed",
    "forbidden",
    "explicitly out",
    "excluded",
    "backlog",
    "does not prove",
    "not prove",
    "out of public wording",
    "without exact fallback",
    "until",
    "target model",
    "открыто",
    "remains",
    "defer",
)

SCAN_EXCLUDED_PARTS = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def read_make_surface(repo: Path) -> str:
    parts = [repo / "Makefile", *sorted((repo / "mk").glob("*.mk"))]
    return "\n".join(read(part) for part in parts if part.exists())


def missing_terms(label: str, text: str, terms: tuple[str, ...]) -> list[str]:
    return [f"{label}: missing required qualifier {term!r}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str) -> list[str]:
    lowered = text.lower()
    return [
        f"{label}: forbidden public overclaim {term!r}"
        for term in FORBIDDEN_PHRASES
        if term.lower() in lowered
    ]


def tracked_markdown(repo: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "*.md"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return []
    paths: list[Path] = []
    for line in result.stdout.splitlines():
        path = Path(line)
        if any(part in SCAN_EXCLUDED_PARTS for part in path.parts):
            continue
        paths.append(repo / path)
    return paths


def risky_claim_errors(repo: Path) -> list[str]:
    errors: list[str] = []
    for path in tracked_markdown(repo):
        label = path.relative_to(repo).as_posix()
        lines = read(path).splitlines()
        for index, line in enumerate(lines):
            lowered = line.lower()
            for term in RISKY_CLAIMS:
                if term.lower() not in lowered:
                    continue
                window = "\n".join(lines[max(0, index - 12) : index + 13]).lower()
                if not any(marker in window for marker in SAFE_CONTEXT_MARKERS):
                    errors.append(f"{label}:{index + 1}: risky claim {term!r} lacks boundary wording")
    return errors


def validate(repo: Path) -> list[str]:
    errors: list[str] = []
    for relative, terms in PUBLIC_DOC_REQUIREMENTS.items():
        text = read(repo / relative)
        errors.extend(missing_terms(relative, text, terms))
        if relative != "docs/PUBLIC_CLAIMS_POLICY.md":
            errors.extend(forbidden_terms(relative, text))
    errors.extend(risky_claim_errors(repo))

    makefile = read_make_surface(repo)
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
        subprocess.run(["git", "init"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "add", "."], cwd=repo, capture_output=True, check=True)
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
    parser.add_argument("--report", default="target/public-claims/report.json")
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
        write_report(repo / args.report, "failed", errors)
        return 1
    write_report(repo / args.report, "passed", [])
    print("public claims check passed")
    return 0


def write_report(path: Path, status: str, errors: list[str]) -> None:
    report = {
        "schema_version": 1,
        "status": status,
        "docs_checked": sorted(PUBLIC_DOC_REQUIREMENTS),
        "risky_claims": list(RISKY_CLAIMS),
        "failures": errors,
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
