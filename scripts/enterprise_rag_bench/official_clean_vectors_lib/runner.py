from __future__ import annotations

import argparse
import os
from typing import Any

from progress_logging import ProgressLogger

from . import logging as vector_logging
from .embedding import embed_texts
from .files import endpoint_origin, load_env_file, write_json
from .texts import document_texts, question_texts, write_vectors


def run(args: argparse.Namespace) -> dict[str, Any]:
    vector_logging.LOGGER = ProgressLogger(
        "official-clean-vectors",
        log_file=getattr(args, "log_file", None),
        status_file=getattr(args, "status_file", None),
    )
    load_env_file(args.env_file)
    args.embedding_url = args.embedding_url or os.environ.get("CORTEXDB_EMBEDDING_URL", "")
    args.embedding_model = args.embedding_model or os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
    args.embedding_api_key = args.embedding_api_key or os.environ.get(
        "CORTEXDB_EMBEDDING_API_KEY", ""
    )
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
        "document_vectors": str(args.output_document_vectors)
        if args.output_document_vectors
        else None,
    }
    vector_logging.LOGGER.status(
        stage="vectors",
        state="running",
        step=1,
        total_steps=4,
        model=args.embedding_model,
        query_vectors=str(args.output_query_vectors) if args.output_query_vectors else None,
        document_vectors=str(args.output_document_vectors)
        if args.output_document_vectors
        else None,
    )

    if args.output_query_vectors:
        qtexts = question_texts(args.questions_file, args.limit_questions)
        vector_logging.log(f"loaded query texts count={len(qtexts)}")
        qvectors = embed_texts(args, qtexts, identity=identity)
        write_vectors(args.output_query_vectors, "question_id", qvectors, scale=args.scale)
        report["query_count"] = len(qvectors)
        vector_logging.log(f"wrote query vectors {args.output_query_vectors}")
        vector_logging.LOGGER.status(
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
        vector_logging.log(f"loaded document texts count={len(dtexts)}")
        dvectors = embed_texts(args, dtexts, identity=identity)
        write_vectors(args.output_document_vectors, "doc_id", dvectors, scale=args.scale)
        report["document_count"] = len(dvectors)
        vector_logging.log(f"wrote document vectors {args.output_document_vectors}")
        vector_logging.LOGGER.status(
            stage="vectors",
            state="running",
            step=3,
            total_steps=4,
            document_count=len(dvectors),
        )

    write_json(args.report, report)
    vector_logging.log(f"wrote report {args.report}")
    vector_logging.LOGGER.status(
        stage="vectors",
        state="done",
        step=4,
        total_steps=4,
        report=str(args.report),
        query_count=report.get("query_count", 0),
        document_count=report.get("document_count", 0),
    )
    return report
