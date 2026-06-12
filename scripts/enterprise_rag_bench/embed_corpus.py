#!/usr/bin/env python3
"""Embed the full EnterpriseRAG-Bench corpus for dense/hybrid retrieval (EPIC-01).

Walks every document in the uuid index, embeds it with the configured embedding
endpoint (bge-m3), and appends ``{"doc_id": ..., "vector": [...]}`` rows to an
output JSONL. The run is **resumable**: doc_ids already present in the output are
skipped, so a killed run can simply be restarted. Progress is logged with a live
ETA so you can see how much is left.

This is the corpus-vector half of EPIC-01. The vectors feed dense candidate
generation / engine-hybrid retrieval, which is what lifts ``semantic`` recall
(lexically-invisible gold docs).

Example
-------
    python3 scripts/enterprise_rag_bench/embed_corpus.py \
      --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
      --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
      --output target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl \
      --env-file .env \
      --report-file target/enterprise-rag-bench/embeddings/corpus_bge_m3_coverage.json \
      --retry-ids-file target/enterprise-rag-bench/embeddings/corpus_bge_m3_retry_ids.txt \
      --workers 8 --batch-size 64 \
      --log-file target/enterprise-rag-bench/embeddings/embed_corpus.log

Backfill only missing ids from the previous coverage report:

    python3 scripts/enterprise_rag_bench/embed_corpus.py \
      --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
      --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
      --output target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl \
      --only-ids-file target/enterprise-rag-bench/embeddings/corpus_bge_m3_retry_ids.txt \
      --env-file .env
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from rerank_with_embeddings import (  # noqa: E402  (local sibling import)
    embedding_request,
    load_doc_text,
    load_env_file,
    read_json,
)


EMBEDDING_ROW_SCHEMA = "cortexdb.embedding_pipeline.vector_row.v2"
EMBEDDING_MANIFEST_SCHEMA = "cortexdb.embedding_pipeline.expected_manifest.v1"


def fmt_duration(seconds: float) -> str:
    seconds = int(max(0, seconds))
    hours, rem = divmod(seconds, 3600)
    minutes, secs = divmod(rem, 60)
    return f"{hours:d}:{minutes:02d}:{secs:02d}"


class Logger:
    """Writes a line to stdout (flushed) and, optionally, to a log file."""

    def __init__(self, log_file: Path | None) -> None:
        self._handle = None
        if log_file is not None:
            log_file.parent.mkdir(parents=True, exist_ok=True)
            self._handle = log_file.open("a", encoding="utf-8")
        self._lock = threading.Lock()

    def log(self, message: str) -> None:
        line = f"[embed-corpus {time.strftime('%Y-%m-%dT%H:%M:%S')}] {message}"
        with self._lock:
            print(line, flush=True)
            if self._handle is not None:
                self._handle.write(line + "\n")
                self._handle.flush()

    def close(self) -> None:
        if self._handle is not None:
            self._handle.close()


def vector_is_valid(vector, expected_dimension: int | None) -> bool:
    if not isinstance(vector, list) or not vector:
        return False
    if expected_dimension is not None and len(vector) != expected_dimension:
        return False
    return True


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def load_done_ids(
    output: Path,
    expected_dimension: int | None = None,
    *,
    expected_model: str | None = None,
    expected_text_hashes: dict[str, str] | None = None,
) -> set[str]:
    """doc_ids already embedded (resume support)."""
    done: set[str] = set()
    if not output.exists():
        return done
    with output.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue  # tolerate a torn final line from a killed run
            doc_id = row.get("doc_id")
            vector = row.get("vector")
            if not isinstance(doc_id, str) or not doc_id:
                continue
            if not vector_is_valid(vector, expected_dimension):
                continue
            if expected_model is not None and row.get("model") != expected_model:
                continue
            expected_hash = expected_text_hashes.get(doc_id) if expected_text_hashes else None
            if expected_hash is not None and row.get("text_hash") != expected_hash:
                continue
            done.add(doc_id)
    return done


def read_only_ids(path: Path, uuid_index: dict) -> list[str]:
    requested: list[str] = []
    seen: set[str] = set()
    unknown: list[str] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        doc_id = raw.strip()
        if not doc_id or doc_id in seen:
            continue
        seen.add(doc_id)
        if doc_id not in uuid_index:
            unknown.append(doc_id)
            continue
        requested.append(doc_id)
    if unknown:
        sample = ", ".join(unknown[:10])
        raise ValueError(f"{path} contains {len(unknown)} unknown doc_ids; sample: {sample}")
    return requested


def embedding_output_report(
    output: Path,
    expected_ids: list[str],
    *,
    model: str,
    min_coverage_bps: int,
    expected_dimension: int | None,
    expected_model: str | None = None,
    expected_text_hashes: dict[str, str] | None = None,
) -> dict:
    expected = set(expected_ids)
    seen: set[str] = set()
    duplicate_ids: list[str] = []
    unexpected_ids: list[str] = []
    invalid_rows: list[str] = []
    duplicate_count = 0
    unexpected_count = 0
    invalid_count = 0
    empty_vector_count = 0
    dimension_mismatch_count = 0
    stale_count = 0
    dimension = expected_dimension
    stale_ids: list[str] = []
    if output.exists():
        with output.open("r", encoding="utf-8") as handle:
            for line_number, line in enumerate(handle, 1):
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError as error:
                    invalid_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: invalid json: {error}")
                    continue
                doc_id = row.get("doc_id")
                vector = row.get("vector")
                if not isinstance(doc_id, str) or not doc_id:
                    invalid_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: missing doc_id")
                    continue
                if not isinstance(vector, list):
                    invalid_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: missing vector for {doc_id}")
                    continue
                if not vector:
                    empty_vector_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: empty vector for {doc_id}")
                    continue
                if dimension is None:
                    dimension = len(vector)
                elif len(vector) != dimension:
                    dimension_mismatch_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(
                            f"line {line_number}: vector dimension {len(vector)} for {doc_id}, expected {dimension}"
                        )
                    continue
                if doc_id not in expected:
                    unexpected_count += 1
                    if len(unexpected_ids) < 25 and doc_id not in unexpected_ids:
                        unexpected_ids.append(doc_id)
                    continue
                if expected_model is not None and row.get("model") != expected_model:
                    stale_count += 1
                    if len(stale_ids) < 25 and doc_id not in stale_ids:
                        stale_ids.append(doc_id)
                    continue
                expected_hash = expected_text_hashes.get(doc_id) if expected_text_hashes else None
                if expected_hash is not None and row.get("text_hash") != expected_hash:
                    stale_count += 1
                    if len(stale_ids) < 25 and doc_id not in stale_ids:
                        stale_ids.append(doc_id)
                    continue
                if doc_id in seen:
                    duplicate_count += 1
                    if len(duplicate_ids) < 25 and doc_id not in duplicate_ids:
                        duplicate_ids.append(doc_id)
                    continue
                seen.add(doc_id)
    missing = sorted(expected - seen)
    coverage_bps = 10_000 if not expected_ids else int(len(seen) * 10_000 / len(expected_ids))
    production_ready = (
        coverage_bps >= min_coverage_bps
        and duplicate_count == 0
        and unexpected_count == 0
        and invalid_count == 0
        and empty_vector_count == 0
        and dimension_mismatch_count == 0
        and stale_count == 0
    )
    return {
        "schema_version": "cortexdb.embedding_pipeline.coverage.v1",
        "model": model,
        "output": str(output),
        "total_items": len(expected_ids),
        "embedded_items": len(seen),
        "missing_items": len(missing),
        "duplicate_items": duplicate_count,
        "unexpected_items": unexpected_count,
        "invalid_rows": invalid_count,
        "empty_vector_rows": empty_vector_count,
        "dimension_mismatch_rows": dimension_mismatch_count,
        "stale_items": stale_count,
        "dimension": dimension,
        "expected_dimension": expected_dimension,
        "expected_model": expected_model,
        "coverage_basis_points": coverage_bps,
        "coverage_percent": coverage_bps / 100.0,
        "min_coverage_basis_points": min_coverage_bps,
        "production_ready": production_ready,
        "missing_ids_sample": missing[:25],
        "duplicate_ids_sample": duplicate_ids,
        "unexpected_ids_sample": unexpected_ids,
        "stale_ids_sample": stale_ids,
        "invalid_row_samples": invalid_rows,
    }


def write_report_and_retry_ids(
    *,
    report_file: Path | None,
    retry_ids_file: Path | None,
    output: Path,
    expected_ids: list[str],
    model: str,
    min_coverage_bps: int,
    expected_dimension: int | None,
    expected_model: str | None = None,
    expected_text_hashes: dict[str, str] | None = None,
) -> None:
    report = embedding_output_report(
        output,
        expected_ids,
        model=model,
        min_coverage_bps=min_coverage_bps,
        expected_dimension=expected_dimension,
        expected_model=expected_model,
        expected_text_hashes=expected_text_hashes,
    )
    if report_file is not None:
        report_file.parent.mkdir(parents=True, exist_ok=True)
        report_file.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if retry_ids_file is not None:
        missing = sorted(
            set(expected_ids)
            - load_done_ids(
                output,
                expected_dimension,
                expected_model=expected_model,
                expected_text_hashes=expected_text_hashes,
            )
        )
        retry_ids_file.parent.mkdir(parents=True, exist_ok=True)
        retry_ids_file.write_text("".join(f"{doc_id}\n" for doc_id in missing), encoding="utf-8")


def load_texts_and_hashes(
    ids: list[str],
    uuid_index: dict,
    sources_dir: Path,
    max_chars: int,
) -> tuple[dict[str, str], dict[str, str]]:
    texts: dict[str, str] = {}
    hashes: dict[str, str] = {}
    for doc_id in ids:
        text = load_doc_text(doc_id, uuid_index, sources_dir, max_chars)
        texts[doc_id] = text
        hashes[doc_id] = text_sha256(text)
    return texts, hashes


def write_expected_manifest(
    path: Path,
    ids: list[str],
    hashes: dict[str, str],
    model: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for doc_id in ids:
            handle.write(
                json.dumps(
                    {
                        "schema_version": EMBEDDING_MANIFEST_SCHEMA,
                        "doc_id": doc_id,
                        "model": model,
                        "text_hash": hashes.get(doc_id),
                    },
                    ensure_ascii=True,
                    sort_keys=True,
                )
                + "\n"
            )


def chunks(items: list, size: int):
    for start in range(0, len(items), size):
        yield items[start : start + size]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="Append-only JSONL of {doc_id, vector}.")
    parser.add_argument("--env-file", type=Path, default=Path(".env"))
    parser.add_argument("--embedding-url", default=None)
    parser.add_argument("--embedding-model", default=None)
    parser.add_argument("--embedding-api-key", default=None)
    parser.add_argument("--batch-size", type=int, default=64, help="Texts per embedding request.")
    parser.add_argument("--workers", type=int, default=8, help="Concurrent embedding requests.")
    parser.add_argument("--max-chars-per-doc", type=int, default=1800)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    parser.add_argument("--retries", type=int, default=4)
    parser.add_argument("--retry-sleep-seconds", type=float, default=2.0)
    parser.add_argument("--limit", type=int, help="Embed at most N new docs (smoke/testing).")
    parser.add_argument("--progress-every", type=int, default=2000, help="Log progress every N embedded docs.")
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--report-file", type=Path, help="Write an embedding coverage report JSON.")
    parser.add_argument("--retry-ids-file", type=Path, help="Write missing doc_ids for retry/backfill.")
    parser.add_argument("--only-ids-file", type=Path, help="Embed only the doc_ids listed in this newline file.")
    parser.add_argument(
        "--track-staleness",
        action="store_true",
        help="Treat rows as stale when model or current document text_hash differs.",
    )
    parser.add_argument("--manifest-file", type=Path, help="Write expected doc_id/text_hash/model JSONL.")
    parser.add_argument("--min-coverage-bps", type=int, default=9950)
    parser.add_argument("--expected-dimension", type=int)
    args = parser.parse_args()

    logger = Logger(args.log_file)
    load_env_file(args.env_file)
    url = args.embedding_url or os.environ.get("CORTEXDB_EMBEDDING_URL", "")
    model = args.embedding_model or os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
    api_key = args.embedding_api_key or os.environ.get("CORTEXDB_EMBEDDING_API_KEY", "")
    if not url or not model or not api_key:
        logger.log("ERROR: embedding url/model/api key missing (set them in --env-file or flags)")
        return 1

    uuid_index = read_json(args.uuid_index)
    all_ids = list(uuid_index.keys())
    run_ids = read_only_ids(args.only_ids_file, uuid_index) if args.only_ids_file else all_ids
    text_cache: dict[str, str] = {}
    expected_text_hashes: dict[str, str] | None = None
    if args.track_staleness or args.manifest_file:
        logger.log(
            f"building text hash manifest ids={len(all_ids)} max_chars={args.max_chars_per_doc}"
        )
        text_cache, expected_text_hashes = load_texts_and_hashes(
            all_ids,
            uuid_index,
            args.sources_dir,
            args.max_chars_per_doc,
        )
        if args.manifest_file:
            write_expected_manifest(args.manifest_file, all_ids, expected_text_hashes, model)
            logger.log(f"wrote expected manifest {args.manifest_file}")
    done = load_done_ids(
        args.output,
        args.expected_dimension,
        expected_model=model if args.track_staleness else None,
        expected_text_hashes=expected_text_hashes if args.track_staleness else None,
    )
    todo = [doc_id for doc_id in run_ids if doc_id not in done]
    if args.limit is not None:
        todo = todo[: args.limit]

    total_corpus = len(all_ids)
    already_total = len(done)
    already_in_scope = sum(1 for doc_id in run_ids if doc_id in done)
    to_embed = len(todo)
    logger.log(
        f"corpus={total_corpus} run_scope={len(run_ids)} already_embedded_total={already_total} "
        f"already_embedded_in_scope={already_in_scope} to_embed={to_embed} "
        f"model={model} workers={args.workers} batch_size={args.batch_size} output={args.output}"
    )
    if to_embed == 0:
        write_report_and_retry_ids(
            report_file=args.report_file,
            retry_ids_file=args.retry_ids_file,
            output=args.output,
            expected_ids=all_ids,
            model=model,
            min_coverage_bps=args.min_coverage_bps,
            expected_dimension=args.expected_dimension,
            expected_model=model if args.track_staleness else None,
            expected_text_hashes=expected_text_hashes if args.track_staleness else None,
        )
        logger.log("nothing to do — corpus already fully embedded")
        logger.close()
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    write_lock = threading.Lock()
    handle = args.output.open("a", encoding="utf-8")

    started = time.time()
    embedded = 0
    failed = 0
    last_logged_at = 0
    counter_lock = threading.Lock()

    def embed_batch(batch_ids: list[str]) -> tuple[list[tuple[str, list[float], str]], int]:
        texts = [
            text_cache.get(doc_id)
            or load_doc_text(doc_id, uuid_index, args.sources_dir, args.max_chars_per_doc)
            for doc_id in batch_ids
        ]
        text_hashes = [
            (expected_text_hashes.get(doc_id) if expected_text_hashes else None) or text_sha256(text)
            for doc_id, text in zip(batch_ids, texts)
        ]
        last_error = None
        for attempt in range(args.retries):
            try:
                vectors = embedding_request(
                    texts, url=url, model=model, api_key=api_key, timeout=args.timeout_seconds
                )
                return list(zip(batch_ids, vectors, text_hashes)), 0
            except Exception as error:  # noqa: BLE001 — keep the run alive on transient errors
                last_error = error
                time.sleep(args.retry_sleep_seconds * (attempt + 1))
        logger.log(f"WARN: batch of {len(batch_ids)} failed after {args.retries} retries: {last_error}")
        return [], len(batch_ids)

    batches = list(chunks(todo, args.batch_size))
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(embed_batch, batch): batch for batch in batches}
        for future in as_completed(futures):
            pairs, batch_failed = future.result()
            with write_lock:
                for doc_id, vector, text_hash in pairs:
                    handle.write(
                        json.dumps(
                            {
                                "schema_version": EMBEDDING_ROW_SCHEMA,
                                "doc_id": doc_id,
                                "model": model,
                                "text_hash": text_hash,
                                "vector": vector,
                            },
                            ensure_ascii=True,
                            sort_keys=True,
                        )
                        + "\n"
                    )
                handle.flush()
            with counter_lock:
                embedded += len(pairs)
                failed += batch_failed
                done_now = embedded + failed
                if args.progress_every and done_now - last_logged_at >= args.progress_every:
                    last_logged_at = done_now
                    elapsed = time.time() - started
                    rate = embedded / elapsed if elapsed > 0 else 0.0
                    remaining = to_embed - done_now
                    eta = remaining / rate if rate > 0 else 0.0
                    pct = done_now / to_embed * 100.0
                    logger.log(
                        f"progress {done_now}/{to_embed} ({pct:.1f}%) | "
                        f"embedded={embedded} failed={failed} | "
                        f"rate={rate:.1f} docs/s | elapsed={fmt_duration(elapsed)} | "
                        f"ETA={fmt_duration(eta)} | total_done={already_total + embedded}/{total_corpus}"
                    )

    handle.close()
    elapsed = time.time() - started
    logger.log(
        f"DONE embedded={embedded} failed={failed} in {fmt_duration(elapsed)} | "
        f"corpus_total_done={already_total + embedded}/{total_corpus} | output={args.output}"
    )
    write_report_and_retry_ids(
        report_file=args.report_file,
        retry_ids_file=args.retry_ids_file,
        output=args.output,
        expected_ids=all_ids,
        model=model,
        min_coverage_bps=args.min_coverage_bps,
        expected_dimension=args.expected_dimension,
        expected_model=model if args.track_staleness else None,
        expected_text_hashes=expected_text_hashes if args.track_staleness else None,
    )
    if failed:
        logger.log(f"NOTE: {failed} docs failed; re-run the same command to retry just those (resumable).")
    logger.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
