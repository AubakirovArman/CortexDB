from __future__ import annotations

import argparse
import json
import math
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from . import logging as vector_logging
from .files import append_jsonl, read_jsonl, text_sha256


def numeric_vector(value: Any, label: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: expected non-empty numeric vector")
    out: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise ValueError(f"{label}: vector values must be numeric")
        number = float(item)
        if not math.isfinite(number):
            raise ValueError(f"{label}: vector values must be finite")
        out.append(number)
    return out


def extract_embeddings(response: dict[str, Any], expected: int) -> list[list[float]]:
    data = response.get("data")
    if isinstance(data, list):
        vectors = [numeric_vector(item.get("embedding"), "data.embedding") for item in data]
        if len(vectors) == expected:
            return vectors
    embeddings = response.get("embeddings")
    if isinstance(embeddings, list) and len(embeddings) == expected:
        return [numeric_vector(item, "embeddings[]") for item in embeddings]
    if expected == 1:
        for key in ("embedding", "vector"):
            if key in response:
                return [numeric_vector(response[key], key)]
    raise ValueError("embedding response did not match requested input count")


def embedding_request(
    texts: list[str],
    *,
    url: str,
    model: str,
    api_key: str,
    timeout: float,
) -> list[list[float]]:
    payload = {"model": model, "input": texts}
    request = urllib.request.Request(
        url,
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        headers={
            "Authorization": "Bearer " + api_key,
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")[:500]
        raise RuntimeError(f"embedding HTTP {error.code}: {detail}") from error
    if not isinstance(body, dict):
        raise RuntimeError("embedding endpoint returned non-object JSON")
    return extract_embeddings(body, len(texts))


class EmbeddingCache:
    def __init__(self, path: Path):
        self.path = path
        self.values: dict[str, list[float]] = {}
        if path.exists():
            for row in read_jsonl(path):
                key = row.get("cache_key")
                vector = row.get("embedding")
                if isinstance(key, str) and isinstance(vector, list):
                    self.values[key] = [float(item) for item in vector]

    def put_many(self, rows: list[tuple[str, list[float]]], identity: dict[str, Any]) -> None:
        out = []
        for key, vector in rows:
            self.values[key] = vector
            out.append(
                {
                    "schema_version": 1,
                    "cache_key": key,
                    "identity": identity,
                    "embedding": vector,
                }
            )
        append_jsonl(self.path, out)


def cache_key(identity: dict[str, Any], text: str) -> str:
    return text_sha256(json.dumps(identity, sort_keys=True) + "\n" + text)


def embed_texts(
    args: argparse.Namespace,
    texts: dict[str, str],
    *,
    identity: dict[str, Any],
) -> dict[str, list[float]]:
    cache = EmbeddingCache(args.cache_file)
    result: dict[str, list[float]] = {}
    missing = []
    for item_id, text in texts.items():
        key = cache_key(identity, text)
        cached = cache.values.get(key)
        if cached is not None:
            result[item_id] = cached
        else:
            missing.append((item_id, key, text))
    vector_logging.log(
        f"embedding texts total={len(texts)} cached={len(result)} missing={len(missing)}"
    )
    vector_logging.LOGGER.progress(
        stage="embed",
        state="running",
        completed=0,
        total=len(missing),
        unit="missing_texts",
        total_texts=len(texts),
        cached_texts=len(result),
        missing_texts=len(missing),
        completed_missing=0,
    )

    for start in range(0, len(missing), args.batch_size):
        batch = missing[start : start + args.batch_size]
        vectors = embedding_request(
            [text for _, _, text in batch],
            url=args.embedding_url,
            model=args.embedding_model,
            api_key=args.embedding_api_key,
            timeout=args.timeout_seconds,
        )
        cache.put_many(
            [(key, vector) for (_, key, _), vector in zip(batch, vectors)],
            identity,
        )
        for (item_id, _, _), vector in zip(batch, vectors):
            result[item_id] = vector
        done = start + len(batch)
        if args.progress_every and (
            done % args.progress_every == 0 or done == len(missing)
        ):
            vector_logging.LOGGER.progress(
                stage="embed",
                state="running",
                completed=done,
                total=len(missing),
                unit="missing_texts",
                total_texts=len(texts),
                cached_texts=len(result) - done,
                missing_texts=len(missing),
                completed_missing=done,
                batch_size=len(batch),
            )
        else:
            vector_logging.LOGGER.status(
                stage="embed",
                state="running",
                total_texts=len(texts),
                cached_texts=len(result) - done,
                missing_texts=len(missing),
                completed_missing=done,
                batch_size=len(batch),
            )
        if args.sleep_seconds > 0:
            time.sleep(args.sleep_seconds)
    return result
