"""Execution pipeline for the corpus embedding CLI."""

from __future__ import annotations

import json
import os
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

from embed_corpus_lib.constants import EMBEDDING_ROW_SCHEMA
from embed_corpus_lib.logging import Logger, fmt_duration
from embed_corpus_lib.manifest import load_texts_and_hashes, write_expected_manifest
from embed_corpus_lib.reporting import write_report_and_retry_ids
from embed_corpus_lib.state import chunks, load_done_ids, read_only_ids, text_sha256
from rerank_with_embeddings import embedding_request, load_doc_text, load_env_file, read_json


def run_embedding(args) -> int:
    logger = Logger(args.log_file)
    try:
        return _run_embedding(args, logger)
    finally:
        logger.close()


def _run_embedding(args, logger: Logger) -> int:
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
        logger.log(f"building text hash manifest ids={len(all_ids)} max_chars={args.max_chars_per_doc}")
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
        _write_final_report(args, all_ids, model, expected_text_hashes)
        logger.log("nothing to do - corpus already fully embedded")
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
            except Exception as error:  # noqa: BLE001
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
                    _write_vector_row(handle, doc_id, model, text_hash, vector)
                handle.flush()
            with counter_lock:
                embedded += len(pairs)
                failed += batch_failed
                last_logged_at = _maybe_log_progress(
                    args,
                    logger,
                    started,
                    to_embed,
                    already_total,
                    total_corpus,
                    embedded,
                    failed,
                    last_logged_at,
                )

    handle.close()
    elapsed = time.time() - started
    logger.log(
        f"DONE embedded={embedded} failed={failed} in {fmt_duration(elapsed)} | "
        f"corpus_total_done={already_total + embedded}/{total_corpus} | output={args.output}"
    )
    _write_final_report(args, all_ids, model, expected_text_hashes)
    if failed:
        logger.log(f"NOTE: {failed} docs failed; re-run the same command to retry just those (resumable).")
    return 0


def _write_vector_row(handle, doc_id: str, model: str, text_hash: str, vector: list[float]) -> None:
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


def _maybe_log_progress(
    args,
    logger: Logger,
    started: float,
    to_embed: int,
    already_total: int,
    total_corpus: int,
    embedded: int,
    failed: int,
    last_logged_at: int,
) -> int:
    done_now = embedded + failed
    if not args.progress_every or done_now - last_logged_at < args.progress_every:
        return last_logged_at
    elapsed = time.time() - started
    rate = embedded / elapsed if elapsed > 0 else 0.0
    remaining = to_embed - done_now
    eta = remaining / rate if rate > 0 else 0.0
    pct = done_now / to_embed * 100.0
    logger.log(
        f"progress {done_now}/{to_embed} ({pct:.1f}%) | embedded={embedded} failed={failed} | "
        f"rate={rate:.1f} docs/s | elapsed={fmt_duration(elapsed)} | ETA={fmt_duration(eta)} | "
        f"total_done={already_total + embedded}/{total_corpus}"
    )
    return done_now


def _write_final_report(args, all_ids: list[str], model: str, expected_text_hashes: dict[str, str] | None) -> None:
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

