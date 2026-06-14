#!/usr/bin/env python3
"""Build an ANN recall corpus from cached BGE-M3 benchmark embeddings."""

from __future__ import annotations

import argparse
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any, Iterable


def load_jsonl(path: Path) -> Iterable[tuple[int, dict[str, Any]]]:
    with path.open("r", encoding="utf-8") as handle:
        for line_no, raw_line in enumerate(handle, 1):
            line = raw_line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_no}: invalid JSON: {error}") from error
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{line_no}: row must be a JSON object")
            yield line_no, row


def numeric_vector(value: Any, label: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: vector must be a non-empty array")
    output: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise ValueError(f"{label}: vector values must be numeric")
        value = float(item)
        if not math.isfinite(value):
            raise ValueError(f"{label}: vector values must be finite")
        output.append(value)
    return output


def quantize(values: list[float], scale: int, normalization: str) -> list[int]:
    if normalization == "unit":
        denom = math.sqrt(sum(value * value for value in values))
    elif normalization == "max_abs":
        denom = max((abs(value) for value in values), default=0.0)
    elif normalization == "none":
        denom = 1.0
    else:
        raise ValueError(f"unknown normalization: {normalization}")
    if denom == 0.0:
        denom = 1.0
    return [max(-32768, min(32767, int(round((value / denom) * scale)))) for value in values]


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
            count += 1
    return count


def doc_rows(args: argparse.Namespace) -> Iterable[dict[str, Any]]:
    for candidate, (line_no, row) in enumerate(load_jsonl(args.corpus_vectors), 1):
        doc_id = row.get("doc_id")
        if not isinstance(doc_id, str) or not doc_id.strip():
            raise ValueError(f"{args.corpus_vectors}:{line_no}: doc_id must be non-empty")
        vector = numeric_vector(row.get("vector"), f"{args.corpus_vectors}:{line_no}")
        yield {
            "candidate": candidate,
            "doc_id": doc_id,
            "vector": quantize(vector, args.scale, args.normalization),
        }
        if candidate >= args.max_documents:
            return


def query_name(row: dict[str, Any], line_no: int) -> str | None:
    key = row.get("key")
    if isinstance(key, str) and key.startswith("q:") and len(key) > 2:
        return key[2:]
    question_id = row.get("question_id")
    if isinstance(question_id, str) and question_id.strip():
        return question_id
    name = row.get("name")
    if isinstance(name, str) and name.strip():
        return name
    return f"query-{line_no}"


def query_rows(args: argparse.Namespace) -> Iterable[dict[str, Any]]:
    count = 0
    for line_no, row in load_jsonl(args.query_cache):
        key = row.get("key")
        if isinstance(key, str) and not key.startswith("q:"):
            continue
        name = query_name(row, line_no)
        if name is None:
            continue
        vector = numeric_vector(row.get("vector"), f"{args.query_cache}:{line_no}")
        count += 1
        yield {
            "name": name,
            "vector": quantize(vector, args.scale, args.normalization),
            "limit": args.limit,
        }
        if count >= args.max_queries:
            return


def build(args: argparse.Namespace) -> dict[str, Any]:
    vectors_path = args.output_dir / "vectors.jsonl"
    queries_path = args.output_dir / "queries.jsonl"
    vector_count = write_jsonl(vectors_path, doc_rows(args))
    query_count = write_jsonl(queries_path, query_rows(args))
    if vector_count == 0:
        raise ValueError(f"{args.corpus_vectors}: no document vectors")
    if query_count == 0:
        raise ValueError(f"{args.query_cache}: no query vectors")
    manifest = {
        "schema_version": 1,
        "corpus_id": args.corpus_id,
        "model": args.model,
        "source": "enterprise-rag-bench-bge-m3-cache",
        "normalization": args.normalization,
        "scale": args.scale,
        "limit": args.limit,
        "vector_count": vector_count,
        "query_count": query_count,
        "vectors": str(vectors_path),
        "queries": str(queries_path),
    }
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return manifest


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus-vectors", type=Path, required=True)
    parser.add_argument("--query-cache", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--model", default="BAAI/bge-m3")
    parser.add_argument("--corpus-id", default="enterprise-rag-bench-bge-m3-cache")
    parser.add_argument("--max-documents", type=int, default=2048)
    parser.add_argument("--max-queries", type=int, default=20)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--scale", type=int, default=32767)
    parser.add_argument("--normalization", choices=["unit", "max_abs", "none"], default="unit")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    for name in ("max_documents", "max_queries", "limit"):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be greater than zero")
    if args.scale <= 0 or args.scale > 32767:
        parser.error("--scale must be in 1..32767")
    return args


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    manifest = build(parse_args(argv))
    sys.stdout.write(json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n")
    return 0


class SelfTests(unittest.TestCase):
    def test_builds_sampled_ann_corpus(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            corpus = root / "corpus.jsonl"
            cache = root / "cache.jsonl"
            corpus.write_text(
                '{"doc_id":"doc-a","vector":[1.0,0.0]}\n'
                '{"doc_id":"doc-b","vector":[0.0,1.0]}\n',
                encoding="utf-8",
            )
            cache.write_text(
                '{"key":"q:alpha","vector":[1.0,0.0]}\n'
                '{"key":"d:doc-a","vector":[1.0,0.0]}\n',
                encoding="utf-8",
            )
            args = parse_args([
                "--corpus-vectors",
                str(corpus),
                "--query-cache",
                str(cache),
                "--output-dir",
                str(root / "out"),
            ])
            manifest = build(args)
            self.assertEqual(manifest["vector_count"], 2)
            self.assertEqual(manifest["query_count"], 1)
            self.assertTrue((root / "out" / "vectors.jsonl").is_file())
            self.assertTrue((root / "out" / "queries.jsonl").is_file())


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
