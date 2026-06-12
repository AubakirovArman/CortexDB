#!/usr/bin/env python3
"""Validate upgrade and migration policy documentation wiring."""

from __future__ import annotations

import sys
from pathlib import Path


REQUIRED_POLICY_TERMS = (
    "v0.1.0-core-alpha",
    "Format Compatibility Matrix",
    "backup-drill",
    "restore",
    "rollback",
    "cortexdb migrate",
    "migration note",
    "read-only compatible",
    "make migration-policy-check",
    "make migration-compatibility-check",
    "make storage-format-change-note-check",
    "storage_format_change_notes_v1.json",
    "release fixture",
    "ACLOGv0",
    "ACS1",
    "ACB0",
    "ACI2",
    "ACI0",
    "ACI1",
    "ACV0",
    "ACH0",
    "ACM0",
)

REQUIRED_STORAGE_LINK_TERMS = (
    "UPGRADE_MIGRATION.md",
    "migration note",
    "storage_format_change_notes_v1.json",
    "storage-format-change-note-check",
)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def require_terms(label: str, text: str, terms: tuple[str, ...]) -> list[str]:
    return [f"{label}: missing {term!r}" for term in terms if term not in text]


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    errors: list[str] = []

    policy = read(repo / "docs/archive/UPGRADE_MIGRATION.md")
    storage = read(repo / "docs/STORAGE_FORMATS.md")
    makefile = read(repo / "Makefile")
    workflow = read(repo / ".github/workflows/rust.yml")

    errors.extend(require_terms("docs/archive/UPGRADE_MIGRATION.md", policy, REQUIRED_POLICY_TERMS))
    errors.extend(require_terms("docs/STORAGE_FORMATS.md", storage, REQUIRED_STORAGE_LINK_TERMS))

    if "migration-policy-check:" not in makefile:
        errors.append("Makefile: missing migration-policy-check target")
    if "migration-compatibility-check:" not in makefile:
        errors.append("Makefile: missing migration-compatibility-check target")
    if "storage-format-change-note-check:" not in makefile:
        errors.append("Makefile: missing storage-format-change-note-check target")
    if "$(MAKE) migration-policy-check" not in makefile:
        errors.append("Makefile: release/alpha gates must run migration-policy-check")
    if "$(MAKE) storage-format-change-note-check" not in makefile:
        errors.append("Makefile: release/alpha gates must run storage-format-change-note-check")
    if "make migration-policy-check" not in workflow:
        errors.append(".github/workflows/rust.yml: CI must run migration-policy-check")
    if "make storage-format-change-note-check" not in workflow:
        errors.append(".github/workflows/rust.yml: CI must run storage-format-change-note-check")

    if errors:
        print("MIGRATION POLICY CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1

    print("migration policy check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
