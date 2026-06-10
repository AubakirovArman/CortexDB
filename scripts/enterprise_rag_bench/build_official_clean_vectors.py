#!/usr/bin/env python3
"""Build clean query/document vectors for engine-hybrid EnterpriseRAG runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

from progress_logging import ProgressLogger


LOGGER = ProgressLogger("official-clean-vectors")


def log(message: str) -> None:
    LOGGER.log(message)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def append_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def load_env_file(path: Path | None) -> None:
    if path is None or not path.exists():
        return
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip().strip("'\""))


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def endpoint_origin(raw_url: str) -> str:
    parsed = urllib.parse.urlparse(raw_url)
    if not parsed.scheme or not parsed.netloc:
        return ""
    return f"{parsed.scheme}://{parsed.netloc}"


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
    log(f"embedding texts total={len(texts)} cached={len(result)} missing={len(missing)}")
    LOGGER.progress(
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
            LOGGER.progress(
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
            LOGGER.status(
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


def quantize_unit_i16(vector: list[float], scale: int) -> list[int]:
    norm = math.sqrt(sum(item * item for item in vector))
    if norm == 0:
        return [0 for _ in vector]
    return [
        max(-32768, min(32767, int(round(item / norm * scale))))
        for item in vector
    ]


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
        parts.append(f"{field}:\n{value}" if len(content_fields) > 1 else str(value))
    return title, "\n\n".join(parts)


def question_texts(path: Path, limit: int | None) -> dict[str, str]:
    rows = read_jsonl(path)
    if limit is not None:
        rows = rows[:limit]
    texts = {}
    for index, row in enumerate(rows, 1):
        qid = str(row.get("question_id") or "")
        question = str(row.get("question") or "")
        if not qid or not question.strip():
            raise ValueError(f"{path}:{index}: missing question_id or question")
        texts[qid] = question
    return texts


def document_texts(
    uuid_index_path: Path,
    sources_dir: Path,
    limit: int | None,
    max_chars: int,
) -> dict[str, str]:
    uuid_index = read_json(uuid_index_path)
    if not isinstance(uuid_index, dict):
        raise ValueError(f"{uuid_index_path}: expected JSON object")
    texts = {}
    started = time.perf_counter()
    total = min(len(uuid_index), limit) if limit is not None else len(uuid_index)
    log(f"load document texts start total={total} max_chars={max_chars}")
    LOGGER.status(
        stage="load_document_texts",
        state="running",
        scanned_documents=0,
        total_documents=total,
        kept_documents=0,
    )
    scanned = 0
    for index, (doc_id, rel_path) in enumerate(uuid_index.items(), 1):
        if limit is not None and len(texts) >= limit:
            break
        scanned = index
        if not isinstance(rel_path, str):
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        text = f"{title}\n\n{content}"[:max_chars].strip()
        if text:
            texts[str(doc_id)] = text
        if index % 50_000 == 0:
            elapsed = max(0.0, time.perf_counter() - started)
            rate = index / elapsed if elapsed > 0 else 0.0
            remaining = max(0, total - index)
            eta = remaining / rate if rate > 0.0 else None
            eta_label = f"{eta:.1f}s" if eta is not None else "unknown"
            log(
                "loaded document texts "
                f"scanned={index}/{total} kept={len(texts)} "
                f"rate={rate:.2f}/s eta={eta_label}"
            )
            LOGGER.status(
                stage="load_document_texts",
                state="running",
                scanned_documents=index,
                total_documents=total,
                kept_documents=len(texts),
                rate_per_second=round(rate, 4),
                eta_seconds=round(eta, 1) if eta is not None else None,
            )
    elapsed = max(0.0, time.perf_counter() - started)
    rate = scanned / elapsed if elapsed > 0.0 else 0.0
    log(f"load document texts done scanned={scanned} kept={len(texts)} rate={rate:.2f}/s")
    LOGGER.status(
        stage="load_document_texts",
        state="done",
        scanned_documents=scanned,
        total_documents=total,
        kept_documents=len(texts),
        rate_per_second=round(rate, 4),
    )
    return texts


def write_vectors(
    path: Path,
    id_field: str,
    vectors: dict[str, list[float]],
    *,
    scale: int,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for item_id, vector in vectors.items():
            row = {
                id_field: item_id,
                "vector": quantize_unit_i16(vector, scale),
            }
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def run(args: argparse.Namespace) -> dict[str, Any]:
    global LOGGER
    LOGGER = ProgressLogger(
        "official-clean-vectors",
        log_file=getattr(args, "log_file", None),
        status_file=getattr(args, "status_file", None),
    )
    load_env_file(args.env_file)
    args.embedding_url = args.embedding_url or os.environ.get("CORTEXDB_EMBEDDING_URL", "")
    args.embedding_model = args.embedding_model or os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
    args.embedding_api_key = args.embedding_api_key or os.environ.get("CORTEXDB_EMBEDDING_API_KEY", "")
    if not args.embedding_url or not args.embedding_model or not args.embedding_api_key:
        raise RuntimeError("embedding url/model/api key are required")

    identity = {
        "provider": "openai-compatible",
        "model": args.embedding_model,
        "endpoint_origin": endpoint_origin(args.embedding_url),
    }
    report: dict[str, Any] = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_vectors.v1",
        "embedding_provider": identity,
        "cache_file": str(args.cache_file),
        "normalization": "unit_i16",
        "scale": args.scale,
        "query_vectors": str(args.output_query_vectors) if args.output_query_vectors else None,
        "document_vectors": str(args.output_document_vectors) if args.output_document_vectors else None,
    }
    LOGGER.status(
        stage="vectors",
        state="running",
        step=1,
        total_steps=4,
        model=args.embedding_model,
        query_vectors=str(args.output_query_vectors) if args.output_query_vectors else None,
        document_vectors=str(args.output_document_vectors) if args.output_document_vectors else None,
    )

    if args.output_query_vectors:
        qtexts = question_texts(args.questions_file, args.limit_questions)
        log(f"loaded query texts count={len(qtexts)}")
        qvectors = embed_texts(args, qtexts, identity=identity)
        write_vectors(args.output_query_vectors, "question_id", qvectors, scale=args.scale)
        report["query_count"] = len(qvectors)
        log(f"wrote query vectors {args.output_query_vectors}")
        LOGGER.status(
            stage="vectors",
            state="running",
            step=2,
            total_steps=4,
            query_count=len(qvectors),
        )

    if args.output_document_vectors:
        dtexts = document_texts(
            args.uuid_index,
            args.sources_dir,
            args.limit_documents,
            args.max_chars_per_doc,
        )
        log(f"loaded document texts count={len(dtexts)}")
        dvectors = embed_texts(args, dtexts, identity=identity)
        write_vectors(args.output_document_vectors, "doc_id", dvectors, scale=args.scale)
        report["document_count"] = len(dvectors)
        log(f"wrote document vectors {args.output_document_vectors}")
        LOGGER.status(
            stage="vectors",
            state="running",
            step=3,
            total_steps=4,
            document_count=len(dvectors),
        )

    write_json(args.report, report)
    log(f"wrote report {args.report}")
    LOGGER.status(
        stage="vectors",
        state="done",
        step=4,
        total_steps=4,
        report=str(args.report),
        query_count=report.get("query_count", 0),
        document_count=report.get("document_count", 0),
    )
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-query-vectors", type=Path)
    parser.add_argument("--output-document-vectors", type=Path)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--cache-file", type=Path, required=True)
    parser.add_argument("--env-file", type=Path, default=Path(".env"))
    parser.add_argument("--embedding-url")
    parser.add_argument("--embedding-model")
    parser.add_argument("--embedding-api-key")
    parser.add_argument("--limit-questions", type=int)
    parser.add_argument("--limit-documents", type=int)
    parser.add_argument("--max-chars-per-doc", type=int, default=1800)
    parser.add_argument("--scale", type=int, default=32767)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--timeout-seconds", type=float, default=60.0)
    parser.add_argument("--sleep-seconds", type=float, default=0.0)
    parser.add_argument("--progress-every", type=int, default=128)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    if not args.output_query_vectors and not args.output_document_vectors:
        parser.error("at least one output vector file is required")
    for name in ("limit_questions", "limit_documents"):
        value = getattr(args, name)
        if value is not None and value <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.batch_size <= 0 or args.max_chars_per_doc <= 0:
        parser.error("--batch-size and --max-chars-per-doc must be positive")
    if args.scale <= 0 or args.scale > 32767:
        parser.error("--scale must be between 1 and 32767")
    return args


def main() -> int:
    try:
        print(json.dumps(run(parse_args()), sort_keys=True))
        return 0
    except Exception as error:
        LOGGER.status(stage="vectors", state="failed", error=str(error))
        raise


if __name__ == "__main__":
    raise SystemExit(main())
