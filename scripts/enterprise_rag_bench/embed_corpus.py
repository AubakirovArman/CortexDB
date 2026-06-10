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
      --workers 8 --batch-size 64 \
      --log-file target/enterprise-rag-bench/embeddings/embed_corpus.log
"""

from __future__ import annotations

import argparse
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


def load_done_ids(output: Path) -> set[str]:
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
            if isinstance(doc_id, str) and doc_id:
                done.add(doc_id)
    return done


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
    done = load_done_ids(args.output)
    todo = [doc_id for doc_id in all_ids if doc_id not in done]
    if args.limit is not None:
        todo = todo[: args.limit]

    total_corpus = len(all_ids)
    already = len(done)
    to_embed = len(todo)
    logger.log(
        f"corpus={total_corpus} already_embedded={already} to_embed={to_embed} "
        f"model={model} workers={args.workers} batch_size={args.batch_size} output={args.output}"
    )
    if to_embed == 0:
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

    def embed_batch(batch_ids: list[str]) -> tuple[list[tuple[str, list[float]]], int]:
        texts = [load_doc_text(doc_id, uuid_index, args.sources_dir, args.max_chars_per_doc) for doc_id in batch_ids]
        last_error = None
        for attempt in range(args.retries):
            try:
                vectors = embedding_request(
                    texts, url=url, model=model, api_key=api_key, timeout=args.timeout_seconds
                )
                return list(zip(batch_ids, vectors)), 0
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
                for doc_id, vector in pairs:
                    handle.write(json.dumps({"doc_id": doc_id, "vector": vector}, ensure_ascii=True) + "\n")
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
                        f"ETA={fmt_duration(eta)} | total_done={already + embedded}/{total_corpus}"
                    )

    handle.close()
    elapsed = time.time() - started
    logger.log(
        f"DONE embedded={embedded} failed={failed} in {fmt_duration(elapsed)} | "
        f"corpus_total_done={already + embedded}/{total_corpus} | output={args.output}"
    )
    if failed:
        logger.log(f"NOTE: {failed} docs failed; re-run the same command to retry just those (resumable).")
    logger.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
