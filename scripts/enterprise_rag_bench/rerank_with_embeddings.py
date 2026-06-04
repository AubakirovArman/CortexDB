#!/usr/bin/env python3
"""Rerank EnterpriseRAG-Bench retrieval rows with an embedding endpoint."""

from __future__ import annotations

import argparse
import json
import math
import os
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def load_env_file(path: Path | None) -> None:
    if path is None or not path.exists():
        return
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip().strip("'\""))


def extract_document_content(doc: dict[str, Any]) -> tuple[str, str]:
    title_field = doc.get("title_field_name")
    content_fields = doc.get("content_field_names")
    title = str(doc.get(title_field, "")) if isinstance(title_field, str) else ""
    if not isinstance(content_fields, list) or not content_fields:
        return title, json.dumps(doc, ensure_ascii=False)
    parts: list[str] = []
    for field in content_fields:
        if not isinstance(field, str) or field not in doc:
            continue
        value = doc[field]
        if isinstance(value, list):
            value = "\n".join(str(item) for item in value)
        elif isinstance(value, dict):
            value = json.dumps(value, ensure_ascii=False)
        parts.append(str(value))
    return title, "\n\n".join(parts)


def load_doc_text(doc_id: str, uuid_index: dict[str, str], sources_dir: Path, max_chars: int) -> str:
    rel_path = uuid_index.get(doc_id)
    if not rel_path:
        return ""
    title, content = extract_document_content(read_json(sources_dir / rel_path))
    return (title + "\n\n" + content)[:max_chars]


def numeric_vector(value: Any, label: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: expected non-empty vector")
    out: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise ValueError(f"{label}: vector values must be numeric")
        numeric = float(item)
        if not math.isfinite(numeric):
            raise ValueError(f"{label}: vector values must be finite")
        out.append(numeric)
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


def embedding_request(texts: list[str], *, url: str, model: str, api_key: str, timeout: float) -> list[list[float]]:
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
                key = row.get("key")
                vector = row.get("vector")
                if isinstance(key, str) and isinstance(vector, list):
                    self.values[key] = [float(item) for item in vector]

    def get_many(self, keys: list[str]) -> list[list[float] | None]:
        return [self.values.get(key) for key in keys]

    def put_many(self, pairs: list[tuple[str, list[float]]]) -> None:
        if not pairs:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("a", encoding="utf-8") as handle:
            for key, vector in pairs:
                self.values[key] = vector
                handle.write(json.dumps({"key": key, "vector": vector}, sort_keys=True) + "\n")


def embed_all(args: argparse.Namespace, cache: EmbeddingCache, texts: dict[str, str]) -> dict[str, list[float]]:
    missing = [(key, text) for key, text in texts.items() if key not in cache.values]
    for start in range(0, len(missing), args.batch_size):
        batch = missing[start : start + args.batch_size]
        vectors = embedding_request(
            [text for _, text in batch],
            url=args.embedding_url,
            model=args.embedding_model,
            api_key=args.embedding_api_key,
            timeout=args.timeout_seconds,
        )
        cache.put_many(list(zip([key for key, _ in batch], vectors)))
        if args.progress_every and (start + len(batch)) % args.progress_every == 0:
            print(f"embedded {start + len(batch)}/{len(missing)} missing texts")
        time.sleep(args.sleep_seconds)
    return cache.values


def cosine(left: list[float], right: list[float]) -> float:
    if len(left) != len(right):
        return -1.0
    dot = sum(a * b for a, b in zip(left, right))
    lnorm = math.sqrt(sum(a * a for a in left))
    rnorm = math.sqrt(sum(b * b for b in right))
    if lnorm == 0 or rnorm == 0:
        return -1.0
    return dot / (lnorm * rnorm)


def run(args: argparse.Namespace) -> dict[str, Any]:
    load_env_file(args.env_file)
    args.embedding_url = args.embedding_url or os.environ.get("CORTEXDB_EMBEDDING_URL", "")
    args.embedding_model = args.embedding_model or os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
    args.embedding_api_key = args.embedding_api_key or os.environ.get("CORTEXDB_EMBEDDING_API_KEY", "")
    if not args.embedding_url or not args.embedding_model or not args.embedding_api_key:
        raise RuntimeError("embedding url/model/api key are required")

    rows = read_jsonl(args.retrieval_file)
    if args.limit is not None:
        rows = rows[: args.limit]
    uuid_index = read_json(args.uuid_index)
    texts: dict[str, str] = {}
    for row in rows:
        qid = str(row.get("question_id", ""))
        texts[f"q:{qid}"] = str(row.get("question", ""))
        for doc_id in row.get("document_ids", []):
            doc_id = str(doc_id)
            texts.setdefault(
                f"d:{doc_id}",
                load_doc_text(doc_id, uuid_index, args.sources_dir, args.max_chars_per_doc),
            )

    cache = EmbeddingCache(args.cache_file)
    vectors = embed_all(args, cache, texts)
    output_rows: list[dict[str, Any]] = []
    for row in rows:
        qid = str(row.get("question_id", ""))
        qvec = vectors[f"q:{qid}"]
        scored = []
        for rank, doc_id in enumerate(row.get("document_ids", [])):
            doc_id = str(doc_id)
            score = cosine(qvec, vectors[f"d:{doc_id}"])
            scored.append((score, rank, doc_id))
        scored.sort(key=lambda item: (-item[0], item[1]))
        output_rows.append({
            **row,
            "document_ids": [doc_id for _, _, doc_id in scored[: args.top_k]],
        })

    write_jsonl(args.output, output_rows)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.embedding_rerank_report.v1",
        "questions": len(output_rows),
        "input": str(args.retrieval_file),
        "output": str(args.output),
        "embedding_model": args.embedding_model,
        "candidate_top_k": max(len(row.get("document_ids", [])) for row in rows) if rows else 0,
        "final_top_k": args.top_k,
        "cache_file": str(args.cache_file),
        "texts_embedded": len(texts),
        "cached_vectors": len(vectors),
    }
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--cache-file", type=Path, required=True)
    parser.add_argument("--env-file", type=Path)
    parser.add_argument("--embedding-url")
    parser.add_argument("--embedding-model")
    parser.add_argument("--embedding-api-key")
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--max-chars-per-doc", type=int, default=1800)
    parser.add_argument("--timeout-seconds", type=float, default=60.0)
    parser.add_argument("--sleep-seconds", type=float, default=0.0)
    parser.add_argument("--progress-every", type=int, default=128)
    args = parser.parse_args()
    if args.top_k <= 0 or args.batch_size <= 0:
        parser.error("--top-k and --batch-size must be positive")
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
