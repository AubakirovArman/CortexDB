#!/usr/bin/env python3
"""Validate machine-readable migration compatibility fixtures."""

from __future__ import annotations

import json
import sys
from pathlib import Path


REQUIRED_FORMATS = {
    "ACLOGv0",
    "ACS2",
    "ACS1",
    "ACB0",
    "ACI3",
    "ACI2",
    "ACI0",
    "ACI1",
    "ACV0",
    "ACH0",
    "ACM0",
}

REQUIRED_BOUNDARIES = {"storage", "api", "sdk"}
CURRENT_RELEASE = "v0.2.0-beta.1"
PREVIOUS_RELEASE = "v0.1.0-core-alpha.5"
REQUIRED_RELEASE_GATES = {
    "restore_gate": "python3 scripts/migration_historical_restore_check.py",
    "upgrade_matrix_v2_gate": "python3 scripts/migration_upgrade_matrix_v2_check.py",
    "api_contract_gate": "make openapi-contract-check",
    "sdk_contract_gate": "make sdk-contract-check",
    "storage_contract_gate": "make storage-compat-check",
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def make_surface(repo: Path) -> str:
    chunks = [read(repo / "Makefile")]
    for path in sorted((repo / "mk").glob("*.mk")):
        chunks.append(read(path))
    return "\n".join(chunks)


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
    if fixture.get("current_release") != CURRENT_RELEASE:
        errors.append(f"fixture current_release must be {CURRENT_RELEASE}")

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

    historical = fixture.get("historical_restore_fixtures", [])
    if not historical:
        errors.append("historical_restore_fixtures must not be empty")
    for index, item in enumerate(historical):
        for field in ("release_tag", "backup_path", "database_path", "fixture", "gate"):
            if not item.get(field):
                errors.append(f"historical_restore_fixtures[{index}] missing {field}")
        for field in ("backup_path", "database_path", "fixture"):
            if item.get(field):
                require_file(repo, item[field], errors)

    release_matrix = fixture.get("release_compatibility_matrix", [])
    if not release_matrix:
        errors.append("release_compatibility_matrix must not be empty")
    for index, item in enumerate(release_matrix):
        if item.get("from") != PREVIOUS_RELEASE:
            errors.append(
                f"release_compatibility_matrix[{index}] from must be {PREVIOUS_RELEASE}"
            )
        if item.get("to") != CURRENT_RELEASE:
            errors.append(f"release_compatibility_matrix[{index}] to must be {CURRENT_RELEASE}")
        for field in ("fixture", "backup_path", "database_path"):
            if not item.get(field):
                errors.append(f"release_compatibility_matrix[{index}] missing {field}")
            else:
                require_file(repo, item[field], errors)
        for field, expected in REQUIRED_RELEASE_GATES.items():
            if item.get(field) != expected:
                errors.append(
                    f"release_compatibility_matrix[{index}] {field} must be {expected!r}"
                )
        if "restore-only" not in item.get("downgrade", ""):
            errors.append(
                f"release_compatibility_matrix[{index}] must document restore-only downgrade"
            )
        for proof in item.get("proof", []):
            require_file(repo, proof, errors)

    docs = {
        "docs/archive/UPGRADE_MIGRATION.md": (
            "migration-compatibility-check",
            "compatibility_matrix_v1.json",
            "upgrade/downgrade matrix",
            "historical restore fixture",
            "previous-release direct database",
            "migration_upgrade_matrix_v2_check.py",
            "v0.1.0-core-alpha.5 -> v0.2.0-beta.1",
        ),
        "docs/archive/BINARY_RELEASES.md": ("binary-release-check", "SHA256SUMS"),
    }
    for path, terms in docs.items():
        text = read(repo / path)
        for term in terms:
            if term not in text:
                errors.append(f"{path}: missing {term!r}")

    makefile = make_surface(repo)
    for term in ("migration-compatibility-check:", "binary-release-check:"):
        if term not in makefile:
            errors.append(f"make surface: missing {term}")

    if errors:
        print("MIGRATION COMPATIBILITY CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1

    print("migration compatibility check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
