#!/usr/bin/env python3
"""Build a deterministic ANN corpus from CortexDB demo/example payloads."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Iterable


DEFAULT_QUERIES = [
    (
        "finance-budget",
        "финансовый бюджет бюджетирование департамент KZT тенге annual budget finance",
    ),
    (
        "investment-projects",
        "инвестиционный проект солнечная электростанция ветропарк capex budget",
    ),
    (
        "finance-kpi",
        "выручка прибыль EBITDA маржа финансовые показатели annual report",
    ),
    (
        "legal-contracts",
        "договор контракт подряд поставка лицензия counterparty legal value",
    ),
    (
        "legal-court-risk",
        "суд решение спор взыскание недействительный договор восстановление",
    ),
    (
        "legal-regulations",
        "кодекс статья налог трудовой regulation law percent",
    ),
    (
        "signatory-authority",
        "подпись полномочия директор signatory authority доверенность",
    ),
    (
        "hr-employee",
        "сотрудник должность директор department hire date employee",
    ),
    (
        "hr-training",
        "обучение курс сертификат training participant duration",
    ),
    (
        "support-wal",
        "support ticket WAL lock restart timeout memory decay scoring",
    ),
    (
        "sec-revenue",
        "SEC filing revenue EBITDA company USD quarterly annual",
    ),
    (
        "world-indicators",
        "country GDP growth indicator year Kazakhstan world bank",
    ),
]

SKIP_DIRS = {"target", "venv", ".venv", "__pycache__", "cortex_db", "cortex_data"}


def tokenize(text: str) -> list[str]:
    return [part.lower() for part in re.findall(r"[\w%.-]+", text, flags=re.UNICODE)]


def stable_hash(value: str) -> int:
    return int.from_bytes(hashlib.blake2b(value.encode("utf-8"), digest_size=8).digest(), "little")


def weighted_tokens(text: str) -> Iterable[tuple[str, int]]:
    seen_metadata = False
    for line in text.splitlines():
        if "=" not in line or line.startswith(" "):
            continue
        key, value = line.split("=", 1)
        key = key.strip().lower()
        value = value.strip().lower()
        if not key or not value:
            continue
        seen_metadata = True
        yield key, 3
        yield f"{key}:{value}", 5
        for token in tokenize(value.replace("_", " ")):
            yield token, 4
            yield f"{key}:{token}", 4
    body_weight = 1 if seen_metadata else 2
    for token in tokenize(text.replace("_", " ")):
        yield token, body_weight


def vectorize(text: str, dimension: int, scale: int) -> list[int]:
    buckets = [0] * dimension
    for token, weight in weighted_tokens(text):
        buckets[stable_hash(token) % dimension] += weight
    max_value = max(buckets, default=0)
    if max_value == 0:
        return [0] * dimension
    return [min(32767, round((value / max_value) * scale)) for value in buckets]


def iter_jsonl_files(source_roots: Iterable[Path]) -> Iterable[Path]:
    for root in source_roots:
        for path in sorted(root.rglob("*.jsonl")):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            yield path


def load_payloads(source_roots: list[Path], max_documents: int | None) -> list[dict]:
    rows: list[dict] = []
    candidate = 1
    for path in iter_jsonl_files(source_roots):
        for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            line = raw_line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_no}: invalid JSON: {error}") from error
            if not isinstance(row, dict):
                continue
            payload = row.get("payload") or row.get("payload_text")
            if not isinstance(payload, str) or not payload.strip():
                payload = json.dumps(row, ensure_ascii=False, sort_keys=True)
            rows.append({
                "candidate": candidate,
                "source": str(path),
                "source_line": line_no,
                "cell_id": row.get("cell_id"),
                "scope": row.get("scope"),
                "payload": payload,
            })
            candidate += 1
            if max_documents is not None and len(rows) >= max_documents:
                return rows
    return rows


def write_jsonl(path: Path, rows: Iterable[dict]) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with path.open("w", encoding="utf-8") as file:
        for row in rows:
            file.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
            count += 1
    return count


def build(args: argparse.Namespace) -> dict:
    documents = load_payloads(args.source_root, args.max_documents)
    if not documents:
        raise ValueError("no demo documents found")
    vectors = [
        {
            "candidate": row["candidate"],
            "vector": vectorize(row["payload"], args.dimension, args.scale),
        }
        for row in documents
    ]
    queries = [
        {
            "name": name,
            "vector": vectorize(text, args.dimension, args.scale),
            "limit": args.limit,
        }
        for name, text in DEFAULT_QUERIES
    ]
    args.output_dir.mkdir(parents=True, exist_ok=True)
    vector_count = write_jsonl(args.output_dir / "vectors.jsonl", vectors)
    query_count = write_jsonl(args.output_dir / "queries.jsonl", queries)
    document_count = write_jsonl(args.output_dir / "documents.jsonl", documents)
    manifest = {
        "schema_version": 1,
        "corpus_id": args.corpus_id,
        "source_roots": [str(path) for path in args.source_root],
        "dimension": args.dimension,
        "scale": args.scale,
        "limit": args.limit,
        "vector_count": vector_count,
        "query_count": query_count,
        "document_count": document_count,
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
    parser.add_argument(
        "--source-root",
        type=Path,
        action="append",
        default=[],
        help="Directory to scan recursively for JSONL rows. May be repeated.",
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--dimension", type=int, default=64)
    parser.add_argument("--scale", type=int, default=1200)
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument("--max-documents", type=int)
    parser.add_argument("--corpus-id", default="cortexdb-demo-domain-v1")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    if not args.source_root:
        args.source_root = [Path("examples/datasets"), Path("examples/rag_demo/data")]
    if args.dimension <= 0:
        parser.error("--dimension must be greater than zero")
    if args.scale <= 0 or args.scale > 32767:
        parser.error("--scale must be in 1..32767")
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
    def test_vectorize_is_deterministic(self) -> None:
        text = "scope=finance\nmetric=budget\n\nbudget finance"
        self.assertEqual(vectorize(text, 16, 1000), vectorize(text, 16, 1000))
        self.assertEqual(len(vectorize(text, 16, 1000)), 16)

    def test_build_from_payload_rows(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            data = root / "data"
            data.mkdir()
            (data / "cells.jsonl").write_text(
                '{"cell_id":1,"payload":"scope=finance\\nstatus=ready\\n\\nBudget text"}\n',
                encoding="utf-8",
            )
            out = root / "out"
            args = parse_args(["--source-root", str(data), "--output-dir", str(out)])
            manifest = build(args)
            self.assertEqual(manifest["vector_count"], 1)
            self.assertGreater(manifest["query_count"], 1)
            self.assertIn('"candidate":1', (out / "vectors.jsonl").read_text(encoding="utf-8"))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
