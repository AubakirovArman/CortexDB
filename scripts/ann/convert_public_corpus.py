#!/usr/bin/env python3
"""Convert public ANN benchmark files into CortexDB ANN JSONL corpus files."""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Iterable


def is_number(value: str) -> bool:
    try:
        float(value)
        return True
    except ValueError:
        return False


def quantize(values: list[float], scale: float, normalization: str) -> list[int]:
    if normalization == "unit":
        denom = math.sqrt(sum(value * value for value in values))
    elif normalization == "max_abs":
        denom = max((abs(value) for value in values), default=0.0)
    else:
        denom = 1.0
    if denom == 0.0:
        denom = 1.0
    result: list[int] = []
    for value in values:
        scaled = int(round((value / denom) * scale))
        result.append(max(-32768, min(32767, scaled)))
    return result


def parse_text_vectors(path: Path, limit: int | None) -> Iterable[tuple[str | None, list[float]]]:
    count = 0
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if not parts:
            continue
        label = None
        raw_values = parts
        if not is_number(parts[0]):
            label = parts[0]
            raw_values = parts[1:]
        if not raw_values:
            raise ValueError(f"{path}:{line_no}: no vector values")
        try:
            values = [float(item) for item in raw_values]
        except ValueError as error:
            raise ValueError(f"{path}:{line_no}: invalid numeric value") from error
        count += 1
        yield label, values
        if limit is not None and count >= limit:
            return


def parse_fvecs(path: Path, limit: int | None) -> Iterable[list[float]]:
    count = 0
    with path.open("rb") as file:
        while True:
            raw_dim = file.read(4)
            if not raw_dim:
                return
            if len(raw_dim) != 4:
                raise ValueError(f"{path}: truncated dimension header")
            (dimension,) = struct.unpack("<i", raw_dim)
            if dimension <= 0:
                raise ValueError(f"{path}: invalid dimension {dimension}")
            raw_vector = file.read(4 * dimension)
            if len(raw_vector) != 4 * dimension:
                raise ValueError(f"{path}: truncated vector")
            values = list(struct.unpack(f"<{dimension}f", raw_vector))
            count += 1
            yield values
            if limit is not None and count >= limit:
                return


def parse_ivecs(path: Path, limit: int | None) -> Iterable[list[int]]:
    count = 0
    with path.open("rb") as file:
        while True:
            raw_dim = file.read(4)
            if not raw_dim:
                return
            if len(raw_dim) != 4:
                raise ValueError(f"{path}: truncated dimension header")
            (dimension,) = struct.unpack("<i", raw_dim)
            if dimension <= 0:
                raise ValueError(f"{path}: invalid dimension {dimension}")
            raw_vector = file.read(4 * dimension)
            if len(raw_vector) != 4 * dimension:
                raise ValueError(f"{path}: truncated ground-truth row")
            values = list(struct.unpack(f"<{dimension}i", raw_vector))
            count += 1
            yield values
            if limit is not None and count >= limit:
                return


def write_jsonl(rows: Iterable[dict], path: Path) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with path.open("w", encoding="utf-8") as file:
        for row in rows:
            file.write(json.dumps(row, separators=(",", ":")) + "\n")
            count += 1
    return count


def vector_rows(args: argparse.Namespace) -> Iterable[dict]:
    source = parse_text_vectors(args.vectors_text, args.max_vectors) if args.vectors_text else (
        (None, values) for values in parse_fvecs(args.vectors_fvecs, args.max_vectors)
    )
    for index, (_label, values) in enumerate(source, 1):
        yield {"candidate": index, "vector": quantize(values, args.scale, args.normalization)}


def query_rows(args: argparse.Namespace) -> Iterable[dict]:
    if args.queries_text:
        source = parse_text_vectors(args.queries_text, args.max_queries)
    elif args.queries_fvecs:
        source = ((None, values) for values in parse_fvecs(args.queries_fvecs, args.max_queries))
    else:
        return
    for index, (label, values) in enumerate(source, 1):
        yield {
            "name": label or f"q{index:06d}",
            "vector": quantize(values, args.scale, args.normalization),
            "limit": args.limit,
        }


def truth_rows(args: argparse.Namespace) -> Iterable[dict]:
    if not args.ground_truth_ivecs:
        return
    offset = 1 if args.ground_truth_base == "zero" else 0
    for index, values in enumerate(parse_ivecs(args.ground_truth_ivecs, args.max_queries), 1):
        candidates = [value + offset for value in values[: args.limit]]
        if any(candidate <= 0 for candidate in candidates):
            raise ValueError("ground truth produced non-positive candidate id")
        yield {"name": f"q{index:06d}", "candidates": candidates}


def convert(args: argparse.Namespace) -> dict:
    args.output_dir.mkdir(parents=True, exist_ok=True)
    vector_count = write_jsonl(vector_rows(args), args.output_dir / "vectors.jsonl")
    query_count = write_jsonl(query_rows(args), args.output_dir / "queries.jsonl")
    truth_count = write_jsonl(truth_rows(args), args.output_dir / "ground_truth.jsonl")
    manifest = {
        "vectors": str(args.vectors_text or args.vectors_fvecs),
        "queries": str(args.queries_text or args.queries_fvecs or ""),
        "ground_truth": str(args.ground_truth_ivecs or ""),
        "output_dir": str(args.output_dir),
        "vector_count": vector_count,
        "query_count": query_count,
        "ground_truth_count": truth_count,
        "limit": args.limit,
        "scale": args.scale,
        "normalization": args.normalization,
        "ground_truth_base": args.ground_truth_base,
    }
    (args.output_dir / "conversion_manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return manifest


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    vector_group = parser.add_mutually_exclusive_group(required=True)
    vector_group.add_argument("--vectors-text", type=Path)
    vector_group.add_argument("--vectors-fvecs", type=Path)
    query_group = parser.add_mutually_exclusive_group()
    query_group.add_argument("--queries-text", type=Path)
    query_group.add_argument("--queries-fvecs", type=Path)
    parser.add_argument("--ground-truth-ivecs", type=Path)
    parser.add_argument("--ground-truth-base", choices=["zero", "one"], default="zero")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--scale", type=float, default=32767.0)
    parser.add_argument("--normalization", choices=["none", "unit", "max_abs"], default="none")
    parser.add_argument("--max-vectors", type=int)
    parser.add_argument("--max-queries", type=int)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    manifest = convert(parse_args(argv))
    sys.stdout.write(json.dumps(manifest, separators=(",", ":")) + "\n")
    return 0


class SelfTests(unittest.TestCase):
    def test_text_conversion(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            (root / "vectors.txt").write_text("a 1.0 0.0\nb 0.0 1.0\n", encoding="utf-8")
            (root / "queries.txt").write_text("qa 1.0 0.0\n", encoding="utf-8")
            args = parse_args([
                "--vectors-text", str(root / "vectors.txt"),
                "--queries-text", str(root / "queries.txt"),
                "--output-dir", str(root / "out"),
                "--normalization", "unit",
                "--limit", "1",
            ])
            manifest = convert(args)
            self.assertEqual(manifest["vector_count"], 2)
            self.assertEqual(manifest["query_count"], 1)
            self.assertIn('"candidate":1', (root / "out" / "vectors.jsonl").read_text())
            self.assertIn('"name":"qa"', (root / "out" / "queries.jsonl").read_text())

    def test_fvecs_and_ivecs_conversion(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            (root / "base.fvecs").write_bytes(struct.pack("<i2f", 2, 1.0, 0.0))
            (root / "query.fvecs").write_bytes(struct.pack("<i2f", 2, 1.0, 0.0))
            (root / "truth.ivecs").write_bytes(struct.pack("<i2i", 2, 0, 1))
            args = parse_args([
                "--vectors-fvecs", str(root / "base.fvecs"),
                "--queries-fvecs", str(root / "query.fvecs"),
                "--ground-truth-ivecs", str(root / "truth.ivecs"),
                "--output-dir", str(root / "out"),
                "--limit", "2",
            ])
            manifest = convert(args)
            self.assertEqual(manifest["ground_truth_count"], 1)
            self.assertIn('"candidates":[1,2]', (root / "out" / "ground_truth.jsonl").read_text())


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
