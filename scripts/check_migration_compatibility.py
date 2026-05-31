#!/usr/bin/env python3
"""Validate machine-readable migration compatibility fixtures."""

from __future__ import annotations

import json
import sys
from pathlib import Path


REQUIRED_FORMATS = {
    "ACLOGv0",
    "ACS1",
    "ACB0",
    "ACI2",
    "ACI0",
    "ACI1",
    "ACV0",
    "ACH0",
    "ACM0",
}

REQUIRED_BOUNDARIES = {"storage", "api", "sdk"}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def require_file(repo: Path, relative: str, errors: list[str]) -> None:
    if not (repo / relative).exists():
        errors.append(f"missing referenced file: {relative}")


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    fixture_path = repo / "fixtures/migration/compatibility_matrix_v1.json"
    errors: list[str] = []

    try:
        fixture = json.loads(read(fixture_path))
    except (AssertionError, json.JSONDecodeError) as error:
        print(f"MIGRATION COMPATIBILITY CHECK FAILED:\n  {error}")
        return 1

    if fixture.get("schema_version") != 1:
        errors.append("fixture schema_version must be 1")
    if fixture.get("release") != "v0.1.0-core-alpha":
        errors.append("fixture release must be v0.1.0-core-alpha")

    formats = {item.get("marker") for item in fixture.get("storage_formats", [])}
    missing_formats = sorted(REQUIRED_FORMATS - formats)
    for marker in missing_formats:
        errors.append(f"missing storage format marker: {marker}")

    boundaries = {item.get("boundary") for item in fixture.get("compatibility_boundaries", [])}
    missing_boundaries = sorted(REQUIRED_BOUNDARIES - boundaries)
    for boundary in missing_boundaries:
        errors.append(f"missing compatibility boundary: {boundary}")

    for section in ("storage_formats", "compatibility_boundaries", "upgrade_matrix"):
        values = fixture.get(section, [])
        if not values:
            errors.append(f"{section} must not be empty")
        for index, item in enumerate(values):
            for proof in item.get("proof", []):
                require_file(repo, proof, errors)
            if section == "upgrade_matrix" and "downgrade" not in item:
                errors.append(f"upgrade_matrix[{index}] must document downgrade behavior")

    docs = {
        "docs/UPGRADE_MIGRATION.md": (
            "migration-compatibility-check",
            "compatibility_matrix_v1.json",
            "upgrade/downgrade matrix",
        ),
        "docs/BINARY_RELEASES.md": ("binary-release-check", "SHA256SUMS"),
    }
    for path, terms in docs.items():
        text = read(repo / path)
        for term in terms:
            if term not in text:
                errors.append(f"{path}: missing {term!r}")

    makefile = read(repo / "Makefile")
    for term in ("migration-compatibility-check:", "binary-release-check:"):
        if term not in makefile:
            errors.append(f"Makefile: missing {term}")

    if errors:
        print("MIGRATION COMPATIBILITY CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1

    print("migration compatibility check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
