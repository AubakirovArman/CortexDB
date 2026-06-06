#!/usr/bin/env python3
"""Embedding provider helpers for ANN benchmark tooling."""

from __future__ import annotations

import hashlib
import json
import math
import os
import shlex
import subprocess
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


DEFAULT_URL_ENV = "CORTEXDB_EMBEDDING_URL"
DEFAULT_MODEL_ENV = "CORTEXDB_EMBEDDING_MODEL"
DEFAULT_KEY_ENV = "CORTEXDB_EMBEDDING_API_KEY"
PROVIDERS = {"command", "openai-compatible", "local", "file", "hash-smoke"}


@dataclass(frozen=True)
class EmbeddingProviderConfig:
    provider: str = "command"
    command: str = ""
    url: str = ""
    url_env: str = DEFAULT_URL_ENV
    model: str = ""
    model_env: str = DEFAULT_MODEL_ENV
    api_key_env: str = DEFAULT_KEY_ENV
    embedding_file: Path | None = None
    timeout_seconds: float = 30.0
    require_model: bool = False
    dimension: int | None = None
    hash_dimension: int = 64


def numeric_vector(value: Any, label: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: expected a non-empty vector array")
    output: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise ValueError(f"{label}: vector values must be numeric")
        if not math.isfinite(float(item)):
            raise ValueError(f"{label}: vector values must be finite")
        output.append(float(item))
    return output


def extract_embedding(response: dict[str, Any]) -> list[float]:
    data = response.get("data")
    if isinstance(data, list) and data:
        first = data[0]
        if isinstance(first, dict) and "embedding" in first:
            return numeric_vector(first["embedding"], "data[0].embedding")
    for key in ("embedding", "vector"):
        if key in response:
            return numeric_vector(response[key], key)
    embeddings = response.get("embeddings")
    if isinstance(embeddings, list) and embeddings:
        return numeric_vector(embeddings[0], "embeddings[0]")
    raise ValueError("embedding response did not contain a supported vector field")


def embed_text(config: EmbeddingProviderConfig, text: str, label: str = "input") -> list[float]:
    if not text.strip():
        raise ValueError(f"{label}: text is empty")
    validate_provider_config(config)
    if config.provider == "command":
        vector = command_embedding(config.command, text, label)
    elif config.provider in {"openai-compatible", "local"}:
        vector = http_embedding(config, text)
    elif config.provider == "file":
        vector = file_embedding(config, text, label)
    elif config.provider == "hash-smoke":
        vector = hash_embedding(text, config.hash_dimension)
    else:
        raise ValueError(f"unsupported embedding provider: {config.provider}")
    validate_dimension(vector, config.dimension, label)
    return vector


def validate_provider_config(config: EmbeddingProviderConfig) -> None:
    if config.provider not in PROVIDERS:
        raise ValueError(f"unsupported embedding provider: {config.provider}")
    if config.timeout_seconds <= 0:
        raise ValueError("embedding timeout must be greater than zero")
    if config.dimension is not None and config.dimension <= 0:
        raise ValueError("embedding dimension must be greater than zero")
    if config.hash_dimension <= 0:
        raise ValueError("hash dimension must be greater than zero")
    if config.provider == "command":
        validate_command(config.command)
    if config.provider in {"openai-compatible", "local"}:
        url = config.url or os.environ.get(config.url_env, "")
        if not url:
            raise ValueError(f"embedding URL is required; set --url or {config.url_env}")
        if config.provider == "openai-compatible" or config.require_model:
            model = config.model or os.environ.get(config.model_env, "")
            if not model:
                raise ValueError(f"embedding model is required; set --model or {config.model_env}")
    if config.provider == "file":
        if config.embedding_file is None:
            raise ValueError("--embedding-file is required when --provider=file")
        if not config.embedding_file.is_file():
            raise ValueError(f"embedding file does not exist: {config.embedding_file}")


def validate_command(command: str) -> None:
    if not command.strip():
        raise ValueError("embedding command is required")
    parts = shlex.split(command)
    if not parts:
        raise ValueError("embedding command is empty")
    if any("hash-smoke" in part for part in parts):
        raise ValueError("real embedding benchmark cannot use hash-smoke provider")


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
    return numeric_vector(value, label)


def validate_dimension(vector: list[float], dimension: int | None, label: str) -> None:
    if dimension is not None and len(vector) != dimension:
        raise ValueError(f"{label}: embedding dimension {len(vector)}, expected {dimension}")


def http_embedding(config: EmbeddingProviderConfig, text: str) -> list[float]:
    url = config.url or os.environ.get(config.url_env, "")
    model = config.model or os.environ.get(config.model_env, "")
    payload: dict[str, Any] = {"input": text}
    if model:
        payload["model"] = model
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    headers = {"Content-Type": "application/json"}
    api_key = os.environ.get(config.api_key_env) if config.api_key_env else ""
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    request = urllib.request.Request(url, data=body, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=config.timeout_seconds) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")[:500]
        raise ValueError(f"embedding endpoint returned HTTP {error.code}: {detail}") from error
    except urllib.error.URLError as error:
        raise ValueError(f"embedding endpoint request failed: {error.reason}") from error
    try:
        decoded = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ValueError("embedding endpoint returned invalid JSON") from error
    if not isinstance(decoded, dict):
        raise ValueError("embedding endpoint response must be a JSON object")
    vector = extract_embedding(decoded)
    return vector


def file_embedding(config: EmbeddingProviderConfig, text: str, label: str) -> list[float]:
    table = load_embedding_file(config.embedding_file)
    digest = text_sha256(text)
    vector = table.get(digest) or table.get(text)
    if vector is None:
        raise ValueError(f"{label}: embedding file has no row for sha256={digest}")
    return vector


def load_embedding_file(path: Path | None) -> dict[str, list[float]]:
    if path is None:
        raise ValueError("embedding file is required")
    table: dict[str, list[float]] = {}
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_no}: invalid JSON: {error}") from error
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{line_no}: row must be a JSON object")
        vector_value = row.get("embedding", row.get("vector"))
        vector = numeric_vector(vector_value, f"{path}:{line_no}:embedding")
        keys = embedding_file_keys(row)
        if not keys:
            raise ValueError(f"{path}:{line_no}: row must include text or text_sha256")
        for key in keys:
            table[key] = vector
    if not table:
        raise ValueError(f"{path}: no embedding rows")
    return table


def embedding_file_keys(row: dict[str, Any]) -> list[str]:
    keys: list[str] = []
    for field in ("text_sha256", "sha256"):
        value = row.get(field)
        if isinstance(value, str) and value.strip():
            keys.append(value.strip())
    for field in ("text", "input", "query", "payload", "payload_text"):
        value = row.get(field)
        if isinstance(value, str) and value.strip():
            keys.append(value)
            keys.append(text_sha256(value))
    return keys


def hash_embedding(text: str, dimension: int) -> list[float]:
    buckets = [0.0] * dimension
    for token in text.replace("_", " ").split():
        digest = hashlib.blake2b(token.lower().encode("utf-8"), digest_size=8).digest()
        buckets[int.from_bytes(digest, "little") % dimension] += 1.0
    return buckets


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def endpoint_origin(raw_url: str | None) -> str:
    if not raw_url:
        return ""
    parsed = urlparse(raw_url)
    if not parsed.scheme or not parsed.netloc:
        return ""
    return f"{parsed.scheme}://{parsed.netloc}"


def provider_profile(config: EmbeddingProviderConfig) -> dict[str, Any]:
    url = config.url or os.environ.get(config.url_env, "")
    model = config.model or os.environ.get(config.model_env, "")
    return {
        "provider": config.provider,
        "model": model,
        "endpoint_origin": endpoint_origin(url),
        "api_key_env": config.api_key_env if config.provider in {"openai-compatible", "local"} else "",
        "api_key_present": bool(os.environ.get(config.api_key_env)) if config.api_key_env else False,
        "embedding_file": str(config.embedding_file or "") if config.provider == "file" else "",
    }
