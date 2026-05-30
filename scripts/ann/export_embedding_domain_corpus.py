#!/usr/bin/env python3
"""Export payload/query text into an embedded-vector ANN corpus source."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Iterable


SKIP_DIRS = {"venv", ".venv", "__pycache__", ".git", "target", "cortex_db", "cortex_data"}


def iter_jsonl_files(source_roots: Iterable[Path]) -> Iterable[Path]:
    for root in source_roots:
        for path in sorted(root.rglob("*.jsonl")):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            yield path


def load_jsonl(path: Path) -> Iterable[tuple[int, dict]]:
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_no}: invalid JSON: {error}") from error
        if isinstance(row, dict):
            yield line_no, row


def row_payload(row: dict) -> str:
    payload = row.get("payload") or row.get("payload_text")
    if isinstance(payload, str) and payload.strip():
        return payload
    return json.dumps(row, ensure_ascii=False, sort_keys=True)


def query_text(row: dict, label: str) -> str:
    for key in ("text", "query", "payload", "payload_text"):
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError(f"{label}: query row must contain text or query")


def command_embedding(command: str, text: str, label: str) -> list[float]:
    try:
        completed = subprocess.run(
            shlex.split(command),
            input=text,
            text=True,
            capture_output=True,
            check=True,
        )
    except subprocess.CalledProcessError as error:
        stderr = error.stderr.strip()
        raise ValueError(f"{label}: embedding command failed: {stderr}") from error
    except FileNotFoundError as error:
        raise ValueError(f"{label}: embedding command not found") from error
    return parse_embedding_json(completed.stdout, label)


def parse_embedding_json(raw_value: str, label: str) -> list[float]:
    try:
        value = json.loads(raw_value)
    except json.JSONDecodeError as error:
        raise ValueError(f"{label}: embedding command must output JSON array") from error
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: embedding must be a non-empty array")
    values: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise ValueError(f"{label}: embedding values must be numeric")
        if not math.isfinite(float(item)):
            raise ValueError(f"{label}: embedding values must be finite")
        values.append(float(item))
    return values


def hash_embedding(text: str, dimension: int) -> list[float]:
    buckets = [0.0] * dimension
    for token in text.replace("_", " ").split():
        digest = hashlib.blake2b(token.lower().encode("utf-8"), digest_size=8).digest()
        buckets[int.from_bytes(digest, "little") % dimension] += 1.0
    return buckets


def quantize(values: list[float], scale: int, normalization: str, label: str) -> list[int]:
    if normalization == "unit":
        denom = math.sqrt(sum(value * value for value in values))
    elif normalization == "max_abs":
        denom = max((abs(value) for value in values), default=0.0)
    elif normalization == "none":
        denom = 1.0
    else:
        raise ValueError(f"{label}: unknown normalization {normalization}")
    if denom == 0.0:
        denom = 1.0
    output = []
    for value in values:
        scaled = int(round((value / denom) * scale))
        output.append(max(-32768, min(32767, scaled)))
    if not output:
        raise ValueError(f"{label}: empty vector")
    return output


def embed_text(args: argparse.Namespace, text: str, label: str) -> list[int]:
    if args.provider == "command":
        if not args.embedding_command:
            raise ValueError("--embedding-command is required when --provider=command")
        raw = command_embedding(args.embedding_command, text, label)
    else:
        raw = hash_embedding(text, args.hash_dimension)
    return quantize(raw, args.scale, args.normalization, label)


def load_documents(args: argparse.Namespace) -> list[dict]:
    rows: list[dict] = []
    candidate = 1
    for path in iter_jsonl_files(args.source_root):
        for line_no, row in load_jsonl(path):
            payload = row_payload(row)
            rows.append({
                "candidate": candidate,
                "cell_id": row.get("cell_id"),
                "scope": row.get("scope"),
                "source": str(path),
                "source_line": line_no,
                "payload": payload,
                "vector": embed_text(args, payload, f"{path}:{line_no}"),
            })
            candidate += 1
            if args.max_documents is not None and len(rows) >= args.max_documents:
                return rows
    if not rows:
        raise ValueError("no source rows found")
    return rows


def load_queries(args: argparse.Namespace, dimension: int) -> list[dict]:
    queries = []
    for line_no, row in load_jsonl(args.queries):
        label = f"{args.queries}:{line_no}"
        name = row.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError(f"{label}: query name must be non-empty")
        limit = row.get("limit", args.limit)
        if not isinstance(limit, int) or limit <= 0:
            raise ValueError(f"{label}: limit must be a positive integer")
        text = query_text(row, label)
        vector = embed_text(args, text, label)
        if len(vector) != dimension:
            raise ValueError(f"{label}: dimension {len(vector)}, expected {dimension}")
        queries.append({"name": name, "text": text, "vector": vector, "limit": limit})
    if not queries:
        raise ValueError(f"{args.queries}: no queries")
    return queries


def write_jsonl(path: Path, rows: Iterable[dict]) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with path.open("w", encoding="utf-8") as file:
        for row in rows:
            file.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
            count += 1
    return count


def export(args: argparse.Namespace) -> dict:
    documents = load_documents(args)
    dimension = len(documents[0]["vector"])
    queries = load_queries(args, dimension)
    payload_dir = args.output_dir / "payloads"
    vector_count = write_jsonl(payload_dir / "cells.jsonl", documents)
    query_count = write_jsonl(args.output_dir / "queries.jsonl", queries)
    manifest = {
        "schema_version": 1,
        "corpus_id": args.corpus_id,
        "provider": args.provider,
        "normalization": args.normalization,
        "scale": args.scale,
        "dimension": dimension,
        "vector_count": vector_count,
        "query_count": query_count,
        "payloads": str(payload_dir),
        "queries": str(args.output_dir / "queries.jsonl"),
    }
    args.output_dir.mkdir(parents=True, exist_ok=True)
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
    parser.add_argument("--provider", choices=["command", "hash-smoke"], default="command")
    parser.add_argument("--embedding-command")
    parser.add_argument("--normalization", choices=["unit", "max_abs", "none"], default="unit")
    parser.add_argument("--scale", type=int, default=32767)
    parser.add_argument("--hash-dimension", type=int, default=64)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--max-documents", type=int)
    parser.add_argument("--corpus-id", default="cortexdb-embedding-domain-v1")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    if args.provider == "command" and not args.embedding_command:
        parser.error("--embedding-command is required when --provider=command")
    if args.scale <= 0 or args.scale > 32767:
        parser.error("--scale must be in 1..32767")
    if args.hash_dimension <= 0:
        parser.error("--hash-dimension must be greater than zero")
    if args.limit <= 0:
        parser.error("--limit must be greater than zero")
    return args


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    manifest = export(parse_args(argv))
    sys.stdout.write(json.dumps(manifest, ensure_ascii=False, separators=(",", ":")) + "\n")
    return 0


class SelfTests(unittest.TestCase):
    def test_parse_embedding_rejects_non_finite_values(self) -> None:
        with self.assertRaises(ValueError):
            parse_embedding_json("[true]", "test")
        with self.assertRaises(ValueError):
            parse_embedding_json("[NaN]", "test")

    def test_hash_smoke_export(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            data = root / "data"
            data.mkdir()
            (data / "cells.jsonl").write_text('{"payload":"alpha budget"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","text":"alpha","limit":1}\n', encoding="utf-8")
            args = parse_args(["--source-root", str(data), "--queries", str(queries), "--output-dir", str(root / "out"), "--provider", "hash-smoke"])
            manifest = export(args)
            self.assertEqual(manifest["provider"], "hash-smoke")
            self.assertTrue((root / "out" / "payloads" / "cells.jsonl").exists())

    def test_command_export(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            helper = root / "embed.py"
            helper.write_text("import json,sys\ntext=sys.stdin.read()\nprint(json.dumps([1,0] if 'alpha' in text else [0,1]))\n", encoding="utf-8")
            data = root / "data"
            data.mkdir()
            (data / "cells.jsonl").write_text('{"payload":"alpha"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","text":"alpha","limit":1}\n', encoding="utf-8")
            command = f"{sys.executable} {helper}"
            args = parse_args(["--source-root", str(data), "--queries", str(queries), "--output-dir", str(root / "out"), "--embedding-command", command])
            self.assertEqual(export(args)["provider"], "command")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
