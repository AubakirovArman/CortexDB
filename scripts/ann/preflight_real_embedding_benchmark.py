#!/usr/bin/env python3
"""Validate inputs for a real embedding ANN benchmark run."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import tempfile
import unittest
from pathlib import Path
from typing import Iterable


SKIP_DIRS = {"venv", ".venv", "__pycache__", ".git", "target", "cortex_db", "cortex_data"}
TEXT_FIELDS = ("payload", "payload_text", "text", "query")
METRICS = {"dot_product", "cosine", "l2"}


def iter_jsonl_files(source_roots: Iterable[Path]) -> Iterable[Path]:
    for root in source_roots:
        if not root.exists():
            raise ValueError(f"source root does not exist: {root}")
        if not root.is_dir():
            raise ValueError(f"source root is not a directory: {root}")
        for path in sorted(root.rglob("*.jsonl")):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            yield path


def load_jsonl(path: Path) -> Iterable[tuple[int, dict]]:
    if not path.exists():
        raise ValueError(f"file does not exist: {path}")
    if not path.is_file():
        raise ValueError(f"path is not a file: {path}")
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_no}: invalid JSON: {error}") from error
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{line_no}: row must be a JSON object")
        yield line_no, value


def has_text(row: dict) -> bool:
    return any(isinstance(row.get(field), str) and row[field].strip() for field in TEXT_FIELDS)


def validate_sources(source_roots: list[Path], max_documents: int | None) -> tuple[int, int]:
    file_count = 0
    row_count = 0
    for path in iter_jsonl_files(source_roots):
        file_count += 1
        for line_no, row in load_jsonl(path):
            if not has_text(row):
                raise ValueError(f"{path}:{line_no}: source row must contain text or payload")
            row_count += 1
            if max_documents is not None and row_count >= max_documents:
                return file_count, row_count
    if file_count == 0:
        raise ValueError("no source .jsonl files found")
    if row_count == 0:
        raise ValueError("no source rows found")
    return file_count, row_count


def validate_queries(path: Path) -> int:
    count = 0
    for line_no, row in load_jsonl(path):
        label = f"{path}:{line_no}"
        name = row.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError(f"{label}: query name must be non-empty")
        if not has_text(row):
            raise ValueError(f"{label}: query row must contain text or query")
        limit = row.get("limit", 1)
        if not isinstance(limit, int) or limit <= 0:
            raise ValueError(f"{label}: limit must be a positive integer")
        count += 1
    if count == 0:
        raise ValueError(f"{path}: no query rows found")
    return count


def validate_command(command: str) -> None:
    if not command.strip():
        raise ValueError("embedding command is required")
    parts = shlex.split(command)
    if not parts:
        raise ValueError("embedding command is empty")
    if any("hash-smoke" in part for part in parts):
        raise ValueError("real embedding benchmark cannot use hash-smoke provider")


def validate_env(required: list[str]) -> None:
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise ValueError("missing required environment variables: " + ", ".join(missing))


def preflight(args: argparse.Namespace) -> dict:
    if args.metric not in METRICS:
        raise ValueError(f"unsupported metric: {args.metric}")
    if args.scale <= 0 or args.scale > 32767:
        raise ValueError("--scale must be in 1..32767")
    if args.limit <= 0:
        raise ValueError("--limit must be greater than zero")
    if args.max_documents is not None and args.max_documents <= 0:
        raise ValueError("--max-documents must be greater than zero")
    validate_command(args.embedding_command)
    validate_env(args.require_env)
    source_file_count, source_row_count = validate_sources(args.source_root, args.max_documents)
    query_count = validate_queries(args.queries)
    report = {
        "ok": True,
        "source_roots": [str(path) for path in args.source_root],
        "source_files": source_file_count,
        "source_rows": source_row_count,
        "queries": str(args.queries),
        "query_rows": query_count,
        "metric": args.metric,
        "scale": args.scale,
        "normalization": args.normalization,
        "limit": args.limit,
        "required_env": sorted(args.require_env),
        "command": args.embedding_command,
    }
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, action="append", required=True)
    parser.add_argument("--queries", type=Path, required=True)
    parser.add_argument("--embedding-command", required=True)
    parser.add_argument("--metric", default="cosine")
    parser.add_argument("--normalization", choices=["unit", "max_abs", "none"], default="unit")
    parser.add_argument("--scale", type=int, default=32767)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--max-documents", type=int)
    parser.add_argument("--require-env", action="append", default=[])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    report = preflight(parse_args(argv))
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def test_preflight_accepts_valid_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            source = root / "source"
            source.mkdir()
            (source / "cells.jsonl").write_text('{"payload":"alpha"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","text":"alpha","limit":1}\n', encoding="utf-8")
            args = parse_args([
                "--source-root",
                str(source),
                "--queries",
                str(queries),
                "--embedding-command",
                "python3 scripts/ann/embed_text_command.py",
            ])
            self.assertEqual(preflight(args)["source_rows"], 1)

    def test_preflight_rejects_missing_env_and_hash_smoke(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            source = root / "source"
            source.mkdir()
            (source / "cells.jsonl").write_text('{"payload":"alpha"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","text":"alpha","limit":1}\n', encoding="utf-8")
            args = parse_args([
                "--source-root",
                str(source),
                "--queries",
                str(queries),
                "--embedding-command",
                "hash-smoke",
                "--require-env",
                "CORTEXDB_TEST_MISSING_ENV",
            ])
            with self.assertRaises(ValueError):
                preflight(args)


if __name__ == "__main__":
    try:
        raise SystemExit(main(__import__("sys").argv[1:]))
    except ValueError as error:
        print(f"error: {error}", file=__import__("sys").stderr)
        raise SystemExit(2)
