#!/usr/bin/env python3
"""Validate storage format migration-note and release-fixture policy wiring."""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any


DEFAULT_NOTES = "fixtures/migration/storage_format_change_notes_v1.json"
DEFAULT_FREEZE = "fixtures/storage/storage_format_freeze_v1.json"
DEFAULT_COMPAT = "fixtures/migration/compatibility_matrix_v1.json"
REQUIRED_GATES = {
    "make storage-format-freeze-check",
    "make storage-format-change-note-check",
    "make migration-policy-check",
    "make migration-compatibility-check",
    "make storage-compat-check",
}
REQUIRED_DOCS = {"docs/STORAGE_FORMATS.md", "docs/archive/UPGRADE_MIGRATION.md"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--notes", default=DEFAULT_NOTES)
    parser.add_argument("--freeze", default=DEFAULT_FREEZE)
    parser.add_argument("--compatibility", default=DEFAULT_COMPAT)
    parser.add_argument("--report", default="target/storage-format-change-notes/report.json")
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_json(repo: Path, relative: str) -> dict[str, Any]:
    path = repo / relative
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{relative}: expected JSON object")
    return payload


def read_text(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def make_surface(repo: Path) -> str:
    chunks = [read_text(repo, "Makefile")]
    for path in sorted((repo / "mk").glob("*.mk")):
        chunks.append(path.read_text(encoding="utf-8"))
    return "\n".join(chunks)


def normalized_marker(value: Any) -> str:
    return str(value or "").replace("\\0", "").replace("\0", "")


def frozen_markers(freeze: dict[str, Any]) -> dict[str, dict[str, str]]:
    markers: dict[str, dict[str, str]] = {}
    for item in freeze.get("formats", []):
        if not isinstance(item, dict):
            continue
        kind = str(item.get("kind") or "")
        extension = "." + str(item.get("extension") or "").lstrip(".")
        current = normalized_marker(item.get("current_magic"))
        if current:
            markers[current] = {
                "kind": kind,
                "extension": extension,
                "compatibility": "current",
            }
        for legacy in item.get("legacy_magics", []):
            marker = normalized_marker(legacy)
            if marker:
                markers[marker] = {
                    "kind": kind,
                    "extension": extension,
                    "compatibility": "read-only compatible",
                }
    return markers


def compatibility_markers(matrix: dict[str, Any]) -> set[str]:
    out = set()
    for item in matrix.get("storage_formats", []):
        if isinstance(item, dict):
            marker = normalized_marker(item.get("marker"))
            if marker:
                out.add(marker)
    return out


def validate_entry(
    repo: Path,
    marker: str,
    expected: dict[str, str],
    entry: dict[str, Any] | None,
    errors: list[str],
) -> None:
    if entry is None:
        errors.append(f"{marker}: missing storage format change-note entry")
        return
    if entry.get("kind") != expected["kind"]:
        errors.append(f"{marker}: kind {entry.get('kind')!r} != {expected['kind']!r}")
    if entry.get("extension") != expected["extension"]:
        errors.append(f"{marker}: extension {entry.get('extension')!r} != {expected['extension']!r}")
    if entry.get("compatibility") != expected["compatibility"]:
        errors.append(
            f"{marker}: compatibility {entry.get('compatibility')!r} != "
            f"{expected['compatibility']!r}"
        )
    policy = str(entry.get("change_policy") or "")
    for term in ("migration note", "fixture"):
        if term not in policy:
            errors.append(f"{marker}: change_policy must mention {term!r}")
    if entry.get("release_fixture_required") is not True:
        errors.append(f"{marker}: release_fixture_required must be true")
    fixture = str(entry.get("release_fixture") or "")
    if not fixture or not (repo / fixture).exists():
        errors.append(f"{marker}: release_fixture does not exist: {fixture}")

    docs = set(str(item) for item in entry.get("required_docs", []))
    missing_docs = sorted(REQUIRED_DOCS - docs)
    for doc in missing_docs:
        errors.append(f"{marker}: required_docs missing {doc}")
    for doc in sorted(docs):
        path = repo / doc
        if not path.exists():
            errors.append(f"{marker}: required doc missing on disk: {doc}")
            continue
        text = path.read_text(encoding="utf-8")
        if marker not in text:
            errors.append(f"{marker}: {doc} does not mention marker")
        if "migration note" not in text:
            errors.append(f"{marker}: {doc} does not mention migration note")

    gates = set(str(item) for item in entry.get("required_gates", []))
    for gate in sorted(REQUIRED_GATES - gates):
        errors.append(f"{marker}: required_gates missing {gate}")


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    notes = read_json(repo, args.notes)
    freeze = read_json(repo, args.freeze)
    matrix = read_json(repo, args.compatibility)
    makefile = make_surface(repo)
    workflow = read_text(repo, ".github/workflows/rust.yml")
    migration_doc = read_text(repo, "docs/archive/UPGRADE_MIGRATION.md")
    storage_doc = read_text(repo, "docs/STORAGE_FORMATS.md")
    errors: list[str] = []

    if notes.get("schema_version") != "cortexdb.storage_format_change_notes.v1":
        errors.append("notes schema_version must be cortexdb.storage_format_change_notes.v1")
    if notes.get("freeze_id") != freeze.get("freeze_id"):
        errors.append("notes freeze_id must match storage format freeze fixture")
    if notes.get("required_change_gate") != "make storage-format-change-note-check":
        errors.append("required_change_gate must be make storage-format-change-note-check")
    if notes.get("required_release_fixture_gate") != "make migration-compatibility-check":
        errors.append("required_release_fixture_gate must be make migration-compatibility-check")

    expected = frozen_markers(freeze)
    matrix_markers = compatibility_markers(matrix)
    if set(expected) - matrix_markers:
        errors.append(
            "compatibility matrix missing markers: " + ", ".join(sorted(set(expected) - matrix_markers))
        )

    entries = notes.get("entries")
    if not isinstance(entries, list):
        errors.append("entries must be a list")
        entries = []
    by_marker: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(entries, 1):
        if not isinstance(item, dict):
            errors.append(f"entries[{index}] must be an object")
            continue
        marker = normalized_marker(item.get("marker"))
        if not marker:
            errors.append(f"entries[{index}] missing marker")
            continue
        if marker in by_marker:
            errors.append(f"{marker}: duplicate entry")
        by_marker[marker] = item

    for marker, expected_entry in sorted(expected.items()):
        validate_entry(repo, marker, expected_entry, by_marker.get(marker), errors)
    extra = sorted(set(by_marker) - set(expected))
    if extra:
        errors.append("notes include markers not in freeze fixture: " + ", ".join(extra))

    for term in (
        "storage-format-change-note-check",
        "storage_format_change_notes_v1.json",
        "release fixture",
    ):
        if term not in migration_doc:
            errors.append(f"docs/archive/UPGRADE_MIGRATION.md missing {term!r}")
    for term in ("storage-format-change-note-check", "storage_format_change_notes_v1.json"):
        if term not in storage_doc:
            errors.append(f"docs/STORAGE_FORMATS.md missing {term!r}")
    if "storage-format-change-note-check:" not in makefile:
        errors.append("make surface missing storage-format-change-note-check target")
    if "$(MAKE) storage-format-change-note-check" not in makefile:
        errors.append("make surface release/alpha gate must run storage-format-change-note-check")
    if "make storage-format-change-note-check" not in workflow:
        errors.append(".github/workflows/rust.yml must run storage-format-change-note-check")

    return {
        "schema_version": "cortexdb.storage_format_change_notes.report.v1",
        "status": "passed" if not errors else "failed",
        "generated_unix_ms": int(time.time() * 1000),
        "notes": args.notes,
        "freeze": args.freeze,
        "compatibility": args.compatibility,
        "marker_count": len(expected),
        "markers": sorted(expected),
        "required_gates": sorted(REQUIRED_GATES),
        "errors": errors,
    }


def main() -> int:
    args = parse_args()
    report = build_report(args)
    output = repo_root() / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        print("storage format change-note check failed: " + "; ".join(report["errors"]))
        return 1
    print(f"storage format change-note check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
