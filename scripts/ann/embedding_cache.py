#!/usr/bin/env python3
"""JSONL embedding cache helpers for ANN benchmark tooling."""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any


def cache_get(path: Path | None, text_sha256: str, identity: dict[str, Any]) -> list[float] | None:
    if path is None or not path.is_file():
        return None
    hit: list[float] | None = None
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(row, dict):
            continue
        if row.get("text_sha256") != text_sha256 or row.get("identity") != identity:
            continue
        vector = numeric_vector(row.get("embedding"))
        if vector is not None:
            hit = vector
    return hit


def cache_put(
    path: Path | None,
    text_sha256: str,
    identity: dict[str, Any],
    vector: list[float],
) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    row = {
        "schema_version": 1,
        "text_sha256": text_sha256,
        "identity": identity,
        "dimension": len(vector),
        "embedding": vector,
    }
    with path.open("a", encoding="utf-8") as file:
        file.write(json.dumps(row, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n")


def cache_identity(
    provider_profile: dict[str, Any],
    configured_dimension: int | None,
    hash_dimension: int,
    command_sha256: str = "",
) -> dict[str, Any]:
    identity = {
        "provider": provider_profile.get("provider", ""),
        "model": provider_profile.get("model", ""),
        "endpoint_origin": provider_profile.get("endpoint_origin", ""),
        "embedding_file": provider_profile.get("embedding_file", ""),
        "configured_dimension": configured_dimension,
        "hash_dimension": hash_dimension,
    }
    if command_sha256:
        identity["command_sha256"] = command_sha256
    return identity


def numeric_vector(value: Any) -> list[float] | None:
    if not isinstance(value, list) or not value:
        return None
    output: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            return None
        number = float(item)
        if not math.isfinite(number):
            return None
        output.append(number)
    return output
