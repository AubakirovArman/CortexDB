#!/usr/bin/env python3
"""Generate ANN ground-truth JSONL from vectors and queries."""

from __future__ import annotations

import argparse
import json
import math
import sys
import unittest
from pathlib import Path
from typing import Iterable


def load_jsonl(path: Path) -> list[dict]:
    rows: list[dict] = []
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_no}: invalid JSON: {error}") from error
    return rows


def validate_vector(vector: object, expected_dim: int | None, label: str) -> tuple[list[int], int]:
    if not isinstance(vector, list) or not vector:
        raise ValueError(f"{label}: vector must be a non-empty array")
    values: list[int] = []
    for item in vector:
        if not isinstance(item, int) or item < -32768 or item > 32767:
            raise ValueError(f"{label}: vector entries must be i16 integers")
        values.append(item)
    if expected_dim is not None and len(values) != expected_dim:
        raise ValueError(f"{label}: dimension {len(values)}, expected {expected_dim}")
    return values, len(values)


def load_vectors(path: Path) -> tuple[dict[int, list[int]], int]:
    vectors: dict[int, list[int]] = {}
    dimension: int | None = None
    for index, row in enumerate(load_jsonl(path), 1):
        candidate = row.get("candidate")
        if not isinstance(candidate, int) or candidate <= 0:
            raise ValueError(f"{path}:{index}: candidate must be positive u32")
        if candidate > 2**32 - 1:
            raise ValueError(f"{path}:{index}: candidate exceeds u32")
        vector, dimension = validate_vector(row.get("vector"), dimension, f"{path}:{index}")
        if candidate in vectors:
            raise ValueError(f"{path}:{index}: duplicate candidate {candidate}")
        vectors[candidate] = vector
    if not vectors:
        raise ValueError(f"{path}: no vectors")
    return vectors, dimension or 0


def load_queries(path: Path, dimension: int) -> list[dict]:
    queries: list[dict] = []
    names: set[str] = set()
    for index, row in enumerate(load_jsonl(path), 1):
        name = row.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError(f"{path}:{index}: query name must be non-empty")
        if name in names:
            raise ValueError(f"{path}:{index}: duplicate query {name}")
        limit = row.get("limit")
        if not isinstance(limit, int) or limit <= 0:
            raise ValueError(f"{path}:{index}: limit must be positive")
        vector, _ = validate_vector(row.get("vector"), dimension, f"{path}:{index}")
        queries.append({"name": name, "vector": vector, "limit": limit})
        names.add(name)
    if not queries:
        raise ValueError(f"{path}: no queries")
    return queries


def score(metric: str, query: list[int], vector: list[int]) -> int:
    if len(query) != len(vector):
        raise ValueError("dimension mismatch")
    if metric == "dot_product":
        return max(0, sum(left * right for left, right in zip(query, vector)))
    if metric == "cosine":
        dot = sum(left * right for left, right in zip(query, vector))
        query_norm = sum(value * value for value in query)
        vector_norm = sum(value * value for value in vector)
        if query_norm == 0 or vector_norm == 0:
            return 0
        norm = math.isqrt(query_norm * vector_norm)
        return 0 if norm == 0 else (abs(dot) * 65535) // norm
    if metric == "l2":
        dist_sq = sum((left - right) ** 2 for left, right in zip(query, vector))
        max_dist = len(query) * 65536 * 65536
        return max(0, max_dist - min(dist_sq, max_dist))
    raise ValueError(f"unknown metric {metric}")


def generate_ground_truth(
    vectors: dict[int, list[int]],
    queries: Iterable[dict],
    metric: str,
) -> list[dict]:
    rows: list[dict] = []
    for query in queries:
        scores = [
            (candidate, score(metric, query["vector"], vector))
            for candidate, vector in vectors.items()
        ]
        scores.sort(key=lambda item: (-item[1], item[0]))
        rows.append({
            "name": query["name"],
            "candidates": [candidate for candidate, _ in scores[: query["limit"]]],
        })
    return rows


def write_jsonl(rows: Iterable[dict], output: Path | None) -> None:
    text = "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows)
    if output is None:
        sys.stdout.write(text)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vectors", type=Path, required=True)
    parser.add_argument("--queries", type=Path, required=True)
    parser.add_argument("--metric", choices=["dot_product", "cosine", "l2"], default="dot_product")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    vectors, dimension = load_vectors(args.vectors)
    queries = load_queries(args.queries, dimension)
    write_jsonl(generate_ground_truth(vectors, queries, args.metric), args.output)
    return 0


class SelfTests(unittest.TestCase):
    def test_dot_product_ground_truth(self) -> None:
        rows = generate_ground_truth(
            {1: [100, 0], 2: [90, 10], 3: [0, 100]},
            [{"name": "q", "vector": [100, 0], "limit": 2}],
            "dot_product",
        )
        self.assertEqual(rows, [{"name": "q", "candidates": [1, 2]}])

    def test_l2_ground_truth(self) -> None:
        rows = generate_ground_truth(
            {1: [100, 0], 2: [0, 100]},
            [{"name": "q", "vector": [95, 5], "limit": 1}],
            "l2",
        )
        self.assertEqual(rows[0]["candidates"], [1])


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
