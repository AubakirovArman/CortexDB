#!/usr/bin/env python3
"""Chunk-level (Parent-Child) corpus embedding for EnterpriseRAG-Bench.

Splits each document into overlapping character windows (children), embeds each,
and writes one row per chunk: {"doc_id": "<parent>#<i>", "parent": "<parent>",
"vector": [...]}. Dense retrieval over these chunks, then mapping chunk->parent,
gives passage-level matching (the relevant span's embedding is distinctive,
unlike a diluted whole-doc average).
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
from rerank_with_embeddings import embedding_request, load_doc_text, load_env_file, read_json  # noqa: E402


def chunk_text(text: str, size: int, overlap: int) -> list[str]:
    if len(text) <= size:
        return [text] if text.strip() else []
    step = max(1, size - overlap)
    out = []
    for start in range(0, len(text), step):
        piece = text[start : start + size]
        if piece.strip():
            out.append(piece)
        if start + size >= len(text):
            break
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--uuid-index", type=Path, required=True)
    ap.add_argument("--sources-dir", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--env-file", type=Path, default=Path(".env"))
    ap.add_argument("--chunk-chars", type=int, default=512)
    ap.add_argument("--chunk-overlap", type=int, default=128)
    ap.add_argument("--max-chars-per-doc", type=int, default=12000)
    ap.add_argument("--max-chunks-per-doc", type=int, default=24)
    ap.add_argument("--batch-size", type=int, default=64)
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--timeout-seconds", type=float, default=120.0)
    ap.add_argument("--limit", type=int)
    ap.add_argument("--progress-every", type=int, default=2000)
    args = ap.parse_args()

    load_env_file(args.env_file)
    url = os.environ.get("CORTEXDB_EMBEDDING_URL", "")
    model = os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
    key = os.environ.get("CORTEXDB_EMBEDDING_API_KEY", "")
    if not url or not model or not key:
        print("ERROR: embedding url/model/key missing"); return 1

    uuid = read_json(args.uuid_index)
    ids = list(uuid.keys())[: args.limit] if args.limit else list(uuid.keys())

    # build (chunk_id, parent, text)
    items: list[tuple[str, str, str]] = []
    for doc_id in ids:
        text = load_doc_text(doc_id, uuid, args.sources_dir, args.max_chars_per_doc)
        chunks = chunk_text(text, args.chunk_chars, args.chunk_overlap)[: args.max_chunks_per_doc]
        for i, ch in enumerate(chunks):
            items.append((f"{doc_id}#{i}", doc_id, ch))

    print(f"docs={len(ids)} chunks={len(items)} chunk_chars={args.chunk_chars}", flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    handle = args.output.open("w", encoding="utf-8")
    lock = threading.Lock()
    started = time.time()
    done = 0

    def work(batch: list[tuple[str, str, str]]):
        vecs = embedding_request([t for _, _, t in batch], url=url, model=model, api_key=key, timeout=args.timeout_seconds)
        return [(cid, par, v) for (cid, par, _), v in zip(batch, vecs)]

    batches = [items[i : i + args.batch_size] for i in range(0, len(items), args.batch_size)]
    with ThreadPoolExecutor(max_workers=args.workers) as ex:
        futs = [ex.submit(work, b) for b in batches]
        for fut in as_completed(futs):
            rows = fut.result()
            with lock:
                for cid, par, v in rows:
                    handle.write(json.dumps({"doc_id": cid, "parent": par, "vector": v}, ensure_ascii=True) + "\n")
                handle.flush()
                done += len(rows)
                if done % args.progress_every < args.batch_size:
                    rate = done / (time.time() - started)
                    print(f"  embedded {done}/{len(items)} chunks ({rate:.0f}/s)", flush=True)
    handle.close()
    print(f"DONE {len(items)} chunks -> {args.output} in {time.time()-started:.0f}s", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
