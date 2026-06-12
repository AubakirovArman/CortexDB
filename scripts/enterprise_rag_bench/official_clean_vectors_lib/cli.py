from __future__ import annotations

import argparse
import json
from pathlib import Path

from . import logging as vector_logging
from .runner import run


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build clean query/document vectors for engine-hybrid EnterpriseRAG runs."
    )
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
        vector_logging.LOGGER.status(stage="vectors", state="failed", error=str(error))
        raise
