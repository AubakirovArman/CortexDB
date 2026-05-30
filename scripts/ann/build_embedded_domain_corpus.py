#!/usr/bin/env python3
"""Build an ANN corpus from payload rows that already contain vectors."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Iterable


SKIP_DIRS = {"venv", ".venv", "__pycache__", "cortex_db", "cortex_data"}


def iter_jsonl_files(source_roots: Iterable[Path]) -> Iterable[Path]:
    for root in source_roots:
        for path in sorted(root.rglob("*.jsonl")):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            yield path


def parse_vector_literal(value: str, label: str) -> list[int]:
    parts = [part for part in value.replace(",", " ").split() if part]
    if not parts:
        raise ValueError(f"{label}: empty vector")
    try:
        vector = [int(part) for part in parts]
    except ValueError as error:
        raise ValueError(f"{label}: vector values must be i16 integers") from error
    return validate_vector(vector, None, label)


def validate_vector(vector: object, dimension: int | None, label: str) -> list[int]:
    if isinstance(vector, str):
        values = parse_vector_literal(vector, label)
    elif isinstance(vector, list):
        values = []
        for item in vector:
            if not isinstance(item, int):
                raise ValueError(f"{label}: vector values must be i16 integers")
            values.append(item)
    else:
        raise ValueError(f"{label}: vector must be an array or string literal")
    if not values:
        raise ValueError(f"{label}: empty vector")
    for item in values:
        if item < -32768 or item > 32767:
            raise ValueError(f"{label}: vector value {item} is outside i16 range")
    if dimension is not None and len(values) != dimension:
        raise ValueError(f"{label}: dimension {len(values)}, expected {dimension}")
    return values


def vector_from_payload(payload: str, key: str, label: str) -> list[int] | None:
    prefix = f"{key}="
    for line in payload.splitlines():
        stripped = line.strip()
        if stripped.startswith(prefix):
            return parse_vector_literal(stripped[len(prefix):], label)
    return None


def row_payload(row: dict) -> str:
    payload = row.get("payload") or row.get("payload_text")
    if isinstance(payload, str) and payload.strip():
        return payload
    return json.dumps(row, ensure_ascii=False, sort_keys=True)


def extract_vector(row: dict, vector_field: str, payload_key: str, label: str) -> list[int] | None:
    if vector_field in row:
        return validate_vector(row[vector_field], None, label)
    return vector_from_payload(row_payload(row), payload_key, label)


def load_documents(args: argparse.Namespace) -> tuple[list[dict], list[dict], int]:
    documents: list[dict] = []
    vectors: list[dict] = []
    dimension: int | None = args.dimension
    candidate = 1
    missing = 0
    queries_path = args.queries.resolve()
    for path in iter_jsonl_files(args.source_root):
        if path.resolve() == queries_path:
            continue
        for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            line = raw_line.strip()
            if not line:
                continue
            label = f"{path}:{line_no}"
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{label}: invalid JSON: {error}") from error
            if not isinstance(row, dict):
                continue
            vector = extract_vector(row, args.vector_field, args.payload_vector_key, label)
            if vector is None:
                missing += 1
                if args.skip_missing_vectors:
                    continue
                raise ValueError(f"{label}: missing vector")
            vector = validate_vector(vector, dimension, label)
            dimension = len(vector)
            documents.append({
                "candidate": candidate,
                "source": str(path),
                "source_line": line_no,
                "cell_id": row.get("cell_id"),
                "scope": row.get("scope"),
                "payload": row_payload(row),
            })
            vectors.append({"candidate": candidate, "vector": vector})
            candidate += 1
            if args.max_documents is not None and len(vectors) >= args.max_documents:
                return documents, vectors, missing
    if not vectors:
        raise ValueError("no rows with vectors found")
    return documents, vectors, missing


def load_queries(path: Path, dimension: int | None, default_limit: int) -> list[dict]:
    queries: list[dict] = []
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line:
            continue
        label = f"{path}:{line_no}"
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{label}: invalid JSON: {error}") from error
        name = row.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError(f"{label}: query name must be non-empty")
        limit = row.get("limit", default_limit)
        if not isinstance(limit, int) or limit <= 0:
            raise ValueError(f"{label}: limit must be a positive integer")
        vector = validate_vector(row.get("vector"), dimension, label)
        queries.append({"name": name, "vector": vector, "limit": limit})
    if not queries:
        raise ValueError(f"{path}: no queries")
    return queries


def write_jsonl(path: Path, rows: Iterable[dict]) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with path.open("w", encoding="utf-8") as file:
        for row in rows:
            file.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
            count += 1
    return count


def build(args: argparse.Namespace) -> dict:
    documents, vectors, missing_vectors = load_documents(args)
    dimension = len(vectors[0]["vector"])
    queries = load_queries(args.queries, dimension, args.limit)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    vector_count = write_jsonl(args.output_dir / "vectors.jsonl", vectors)
    query_count = write_jsonl(args.output_dir / "queries.jsonl", queries)
    document_count = write_jsonl(args.output_dir / "documents.jsonl", documents)
    manifest = {
        "schema_version": 1,
        "corpus_id": args.corpus_id,
        "source_roots": [str(path) for path in args.source_root],
        "queries_source": str(args.queries),
        "dimension": dimension,
        "limit": args.limit,
        "vector_count": vector_count,
        "query_count": query_count,
        "document_count": document_count,
        "missing_vectors_skipped": missing_vectors if args.skip_missing_vectors else 0,
        "vectors": str(args.output_dir / "vectors.jsonl"),
        "queries": str(args.output_dir / "queries.jsonl"),
        "documents": str(args.output_dir / "documents.jsonl"),
    }
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return manifest


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, action="append", required=True)
    parser.add_argument("--queries", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--vector-field", default="vector")
    parser.add_argument("--payload-vector-key", default="vector")
    parser.add_argument("--dimension", type=int)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--max-documents", type=int)
    parser.add_argument("--skip-missing-vectors", action="store_true")
    parser.add_argument("--corpus-id", default="cortexdb-embedded-domain-v1")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    if args.dimension is not None and args.dimension <= 0:
        parser.error("--dimension must be greater than zero")
    if args.limit <= 0:
        parser.error("--limit must be greater than zero")
    if args.max_documents is not None and args.max_documents <= 0:
        parser.error("--max-documents must be greater than zero")
    return args


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    manifest = build(parse_args(argv))
    sys.stdout.write(json.dumps(manifest, ensure_ascii=False, separators=(",", ":")) + "\n")
    return 0


class SelfTests(unittest.TestCase):
    def test_embedded_vectors_build_corpus(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            data = root / "data"
            data.mkdir()
            (data / "cells.jsonl").write_text(
                '{"cell_id":1,"payload":"scope=x\\nvector=10,0\\nalpha"}\n'
                '{"cell_id":2,"payload":"scope=x\\nvector=0,10\\nbeta"}\n',
                encoding="utf-8",
            )
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"alpha","vector":[10,0],"limit":1}\n', encoding="utf-8")
            out = root / "out"
            args = parse_args(["--source-root", str(data), "--queries", str(queries), "--output-dir", str(out)])
            manifest = build(args)
            self.assertEqual(manifest["vector_count"], 2)
            self.assertIn('"candidate":1', (out / "vectors.jsonl").read_text(encoding="utf-8"))

    def test_missing_vector_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            data = root / "data"
            data.mkdir()
            (data / "cells.jsonl").write_text('{"payload":"no vector"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","vector":[1],"limit":1}\n', encoding="utf-8")
            args = parse_args(["--source-root", str(data), "--queries", str(queries), "--output-dir", str(root / "out")])
            with self.assertRaisesRegex(ValueError, "missing vector"):
                build(args)

    def test_query_file_is_not_scanned_as_document(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            data = root / "data"
            data.mkdir()
            (data / "cells.jsonl").write_text(
                '{"cell_id":1,"vector":[10,0],"payload":"alpha"}\n',
                encoding="utf-8",
            )
            queries = data / "queries.jsonl"
            queries.write_text('{"name":"alpha","vector":[10,0],"limit":1}\n', encoding="utf-8")
            args = parse_args(["--source-root", str(data), "--queries", str(queries), "--output-dir", str(root / "out")])
            manifest = build(args)
            self.assertEqual(manifest["vector_count"], 1)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
