#!/usr/bin/env python3
"""Validate the Storage Format Freeze v1 contract."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import time
from pathlib import Path
from typing import Any


FIXTURE = "fixtures/storage/storage_format_freeze_v1.json"
REQUIRED_KINDS = {
    "aclog_wal",
    "segment",
    "bitmap_index",
    "lexical_index",
    "vector_index",
    "hnsw_graph",
    "manifest",
}
RUST_MAGIC_CONSTANTS = {
    "segment": ("crates/cortex-storage/src/format.rs", "SEGMENT_MAGIC"),
    "bitmap_index": ("crates/cortex-storage/src/format.rs", "BITMAP_INDEX_MAGIC"),
    "lexical_index": ("crates/cortex-storage/src/format.rs", "LEXICAL_INDEX_MAGIC"),
    "vector_index": ("crates/cortex-storage/src/format.rs", "VECTOR_INDEX_MAGIC"),
    "hnsw_graph": ("crates/cortex-storage/src/format.rs", "HNSW_GRAPH_MAGIC"),
    "manifest": ("crates/cortex-storage/src/format.rs", "MANIFEST_MAGIC"),
    "aclog_wal": ("crates/cortex-storage/src/wal/record.rs", "ACLOG_MAGIC"),
}
RUST_SPEC_NAMES = {
    "segment": "Segment",
    "bitmap_index": "Bitmap index",
    "lexical_index": "Lexical index",
    "vector_index": "Vector index",
    "hnsw_graph": "HNSW graph",
    "manifest": "Manifest",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", default=FIXTURE)
    parser.add_argument("--report", default="target/storage-format-freeze/report.json")
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_text(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def read_json(repo: Path, relative: str) -> dict[str, Any]:
    value = json.loads(read_text(repo, relative))
    if not isinstance(value, dict):
        raise ValueError(f"{relative} must contain a JSON object")
    return value


def git_sha(repo: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def rust_magic_literal(repo: Path, kind: str) -> str:
    path, const_name = RUST_MAGIC_CONSTANTS[kind]
    text = read_text(repo, path)
    pattern = rf"pub const {const_name}: [^=]+ = \*b\"([^\"]+)\";"
    match = re.search(pattern, text)
    if not match:
        raise ValueError(f"{path}: missing {const_name}")
    return match.group(1).replace("\\0", "\\0")


def rust_version(repo: Path, kind: str, fallback: int) -> int:
    if kind == "aclog_wal":
        text = read_text(repo, "crates/cortex-storage/src/wal/record.rs")
        match = re.search(r"pub const WAL_FORMAT_VERSION: u16 = (\d+);", text)
        if not match:
            raise ValueError("wal/record.rs: missing WAL_FORMAT_VERSION")
        return int(match.group(1))
    text = read_text(repo, "crates/cortex-storage/src/format.rs")
    name = RUST_SPEC_NAMES[kind]
    block_match = re.search(
        rf'name: "{re.escape(name)}",.*?current_version: (\d+),',
        text,
        re.S,
    )
    version_match = block_match
    if not version_match:
        return fallback
    return int(version_match.group(1))


def check_docs(repo: Path, item: dict[str, Any], errors: list[str]) -> None:
    magic = str(item["current_magic"])
    extension = "." + str(item["extension"])
    for doc in item.get("docs", []):
        path = repo / doc
        if not path.exists():
            errors.append(f"{doc}: missing")
            continue
        text = path.read_text(encoding="utf-8")
        visible_magic = magic.replace("\\0", "")
        if visible_magic not in text:
            errors.append(f"{doc}: missing current magic {visible_magic}")
        if doc.endswith(("STORAGE_FORMATS.md", "UPGRADE_MIGRATION.md")) and extension not in text:
            errors.append(f"{doc}: missing extension {extension}")
    for legacy in item.get("legacy_magics", []):
        if legacy not in read_text(repo, "docs/STORAGE_FORMATS.md"):
            errors.append(f"docs/STORAGE_FORMATS.md: missing legacy magic {legacy}")


def check_fixture(repo: Path, fixture: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if fixture.get("schema_version") != "cortexdb.storage_format_freeze.v1":
        errors.append("fixture schema_version must be cortexdb.storage_format_freeze.v1")
    if fixture.get("freeze_id") != "storage-format-freeze-v1":
        errors.append("fixture freeze_id must be storage-format-freeze-v1")
    formats = fixture.get("formats")
    if not isinstance(formats, list):
        return ["formats must be a list"]
    kinds = {item.get("kind") for item in formats if isinstance(item, dict)}
    for missing in sorted(REQUIRED_KINDS - kinds):
        errors.append(f"missing frozen format kind: {missing}")
    if len(formats) != len(REQUIRED_KINDS):
        errors.append(f"expected {len(REQUIRED_KINDS)} frozen formats")
    for item in formats:
        if not isinstance(item, dict):
            errors.append("format entry must be an object")
            continue
        kind = str(item.get("kind"))
        magic = str(item.get("current_magic"))
        version = int(item.get("current_version", -1))
        if kind not in REQUIRED_KINDS:
            errors.append(f"unexpected format kind: {kind}")
            continue
        try:
            rust_magic = rust_magic_literal(repo, kind)
            rust_current_version = rust_version(repo, kind, version)
        except ValueError as error:
            errors.append(str(error))
            continue
        if rust_magic != magic:
            errors.append(f"{kind}: fixture magic {magic!r} != Rust {rust_magic!r}")
        if rust_current_version != version:
            errors.append(f"{kind}: fixture version {version} != Rust {rust_current_version}")
        if not item.get("breaking_change_policy"):
            errors.append(f"{kind}: missing breaking_change_policy")
        check_docs(repo, item, errors)
    gates = set(fixture.get("required_evidence_gates", []))
    for gate in (
        "make storage-format-freeze-check",
        "make migration-compatibility-check",
        "make storage-compat-check",
    ):
        if gate not in gates:
            errors.append(f"missing required evidence gate: {gate}")
    return errors


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    fixture = read_json(repo, args.fixture)
    errors = check_fixture(repo, fixture)
    return {
        "schema_version": "cortexdb.storage_format_freeze.report.v1",
        "status": "passed" if not errors else "failed",
        "git_sha": git_sha(repo),
        "generated_unix_ms": int(time.time() * 1000),
        "fixture": args.fixture,
        "format_count": len(fixture.get("formats", [])),
        "frozen_kinds": [item.get("kind") for item in fixture.get("formats", [])],
        "companion_gates": fixture.get("required_evidence_gates", []),
        "boundary": {
            "proves": [
                "current storage format markers are frozen in a machine-readable contract",
                "Rust storage constants match the freeze contract",
                "storage format docs and migration policy mention every frozen marker",
            ],
            "does_not_prove": [
                "online rolling upgrade",
                "in-place downgrade",
                "renumbering existing format markers to v1",
            ],
        },
        "errors": errors,
    }


def main() -> int:
    args = parse_args()
    report = build_report(args)
    output = repo_root() / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        print("storage format freeze check failed: " + "; ".join(report["errors"]))
        return 1
    print(f"storage format freeze check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
