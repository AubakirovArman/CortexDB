#!/usr/bin/env python3
"""Embed the full EnterpriseRAG-Bench corpus for dense/hybrid retrieval.

The run is resumable: doc_ids already present in the output JSONL are skipped,
so a killed run can be restarted with the same command. Progress is logged with
a live ETA.

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
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from embed_corpus_lib.runner import run_embedding  # noqa: E402


def parse_args() -> argparse.Namespace:
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
    return parser.parse_args()


def main() -> int:
    return run_embedding(parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
