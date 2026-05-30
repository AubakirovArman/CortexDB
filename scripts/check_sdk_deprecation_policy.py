#!/usr/bin/env python3
"""Validate SDK/API deprecation policy and breaking-change documentation."""

from __future__ import annotations

import re
import sys
from pathlib import Path


LEGACY_ROUTE_RE = re.compile(r"^[ ]{2}(/[^:]+):\s*$")
SDK_SOURCE_GLOBS = (
    "sdk/python/*.py",
    "sdk/typescript/*.{js,cjs,ts,d.ts}",
    "crates/cortex-sdk/src/*.rs",
)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def normalized(value: str) -> str:
    return re.sub(r"\s+", " ", value)


def deprecated_openapi_paths(openapi: str) -> list[str]:
    paths: list[str] = []
    current_path: str | None = None
    for line in openapi.splitlines():
        match = LEGACY_ROUTE_RE.match(line)
        if match:
            current_path = match.group(1)
        if "deprecated: true" in line and current_path:
            paths.append(current_path)
    return paths


def sdk_source_files(repo: Path) -> list[Path]:
    files: list[Path] = []
    for pattern in SDK_SOURCE_GLOBS:
        files.extend(repo.glob(pattern))
    return sorted(path for path in files if path.is_file())


def validate_deprecation_policy(repo: Path, errors: list[str]) -> None:
    policy_path = repo / "docs/SDK_DEPRECATION_POLICY.md"
    policy = read_text(policy_path)
    policy_norm = normalized(policy)
    api_changelog = read_text(repo / "docs/API_CHANGELOG.md")
    compatibility = read_text(repo / "docs/API_COMPATIBILITY.md")
    openapi = read_text(repo / "docs/openapi.yaml")
    paths = deprecated_openapi_paths(openapi)
    if not paths:
        errors.append("docs/openapi.yaml: expected deprecated legacy routes to be listed")
        return
    for phrase in (
        "minimum deprecation window",
        "version bump",
        "CHANGELOG.md",
        "docs/API_CHANGELOG.md",
        "SDK clients MUST NOT expose deprecated compatibility aliases",
    ):
        if phrase not in policy_norm:
            errors.append(f"docs/SDK_DEPRECATION_POLICY.md: missing {phrase!r}")
    for path in paths:
        if path not in policy:
            errors.append(f"docs/SDK_DEPRECATION_POLICY.md: missing deprecated route {path}")
        if path not in api_changelog:
            errors.append(f"docs/API_CHANGELOG.md: missing deprecated route {path}")
        if path not in compatibility:
            errors.append(f"docs/API_COMPATIBILITY.md: missing deprecated route {path}")


def validate_sdk_sources(repo: Path, errors: list[str]) -> None:
    forbidden = {
        '"/get"',
        '"/put"',
        '"/flush"',
        '"/tombstone"',
        "'/get'",
        "'/put'",
        "'/flush'",
        "'/tombstone'",
    }
    for path in sdk_source_files(repo):
        text = read_text(path)
        for token in forbidden:
            if token in text:
                errors.append(f"{path.relative_to(repo)}: SDK source uses deprecated route alias {token}")


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    errors: list[str] = []
    try:
        validate_deprecation_policy(repo, errors)
        validate_sdk_sources(repo, errors)
    except FileNotFoundError as exc:
        errors.append(str(exc))
    if errors:
        print("SDK DEPRECATION POLICY CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1
    print("OK: SDK deprecation policy is consistent.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
