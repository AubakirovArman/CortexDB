"""Shared helpers for the C20 baseline comparison gate."""

from __future__ import annotations

import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

Q16_ONE = 65_535
VECTOR_DIM = 96
TOKEN_RE = re.compile(r"[0-9A-Za-zА-Яа-яЁёІіҢңҒғҮүҰұҚқӨөҺһ]+")


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{line_number}: expected JSON object")
            rows.append(row)
    return rows


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def tokens(text: str) -> list[str]:
    return [match.group(0).lower() for match in TOKEN_RE.finditer(text)]


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def mean_q16(values: list[int]) -> int:
    if not values:
        return Q16_ONE
    return min(Q16_ONE, sum(values) // len(values))


def q16_pct(value: int) -> str:
    return f"{(value / Q16_ONE) * 100:.2f}%"


def p95(values: list[int]) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    index = min(len(ordered) - 1, math.ceil(len(ordered) * 0.95) - 1)
    return ordered[index]


def query_id(row: dict[str, Any]) -> str:
    for key in ("query_id", "name"):
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError("query row missing query_id/name")


def query_text(row: dict[str, Any]) -> str:
    for key in ("query", "text", "name"):
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError(f"{row.get('query_id', '<query>')}: missing query text")


def chunk_text(row: dict[str, Any]) -> str:
    title = str(row.get("title", ""))
    body = str(row.get("text") or row.get("payload") or "")
    return f"{title}\n{body}".strip()


def repo_path(root: Path, value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else root / path


def reciprocal_rank_q16(top: list[str], relevant: set[str]) -> int:
    if not relevant:
        return Q16_ONE
    for rank, chunk_id in enumerate(top, start=1):
        if chunk_id in relevant:
            return max(1, Q16_ONE // rank)
    return 0


def ndcg_q16(top: list[str], relevant: set[str]) -> int:
    if not relevant:
        return Q16_ONE
    gains = [
        1.0 / math.log2(rank + 1)
        for rank, chunk_id in enumerate(top, start=1)
        if chunk_id in relevant
    ]
    ideal_count = min(len(relevant), len(top))
    if ideal_count == 0:
        return Q16_ONE
    ideal = sum(1.0 / math.log2(rank + 1) for rank in range(1, ideal_count + 1))
    return min(Q16_ONE, int((sum(gains) / ideal) * Q16_ONE)) if ideal else 0


def fts_query(text: str) -> str:
    terms = tokens(text)
    if not terms:
        return ""
    return " OR ".join(f'"{term.replace(chr(34), chr(34) + chr(34))}"' for term in terms[:24])


def vectorize(text: str) -> list[float]:
    vector = [0.0] * VECTOR_DIM
    for token in tokens(text):
        digest = hashlib.blake2b(token.encode("utf-8"), digest_size=8).digest()
        bucket = int.from_bytes(digest[:4], "little") % VECTOR_DIM
        sign = 1.0 if digest[4] & 1 else -1.0
        vector[bucket] += sign
    norm = math.sqrt(sum(value * value for value in vector))
    if norm <= 0.0:
        return vector
    return [value / norm for value in vector]


def dot(left: list[float], right: list[float]) -> float:
    return sum(a * b for a, b in zip(left, right))
