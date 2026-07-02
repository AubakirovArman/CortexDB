"""Retrieval-stage orchestration."""

from __future__ import annotations

import argparse
from pathlib import Path

from .constants import ROOT
from .status import log, run_cmd


def retrieve(args: argparse.Namespace, p: dict[str, Path]) -> None:
    retrieval_top_k = (
        args.embedding_rerank_candidates if args.embedding_rerank else args.top_k
    )
    sanitize_target = (
        p["clean_retrieval_wide"] if args.embedding_rerank else p["clean_retrieval"]
    )
    log(
        "retrieve config "
        f"mode={args.retrieval_mode} rerank={args.rerank} "
        f"embedding_rerank={args.embedding_rerank} top_k={retrieval_top_k} "
        f"batch_size={args.batch_size} "
        f"reuse_db={args.reuse_db} db_root={p['db_root']} "
        f"query_vectors={args.query_vectors} document_vectors={args.document_vectors} "
        f"prefilter_retrieval={args.prefilter_retrieval} "
        f"progress_every={args.retrieval_progress_every}"
    )
    # Build skipped: rely on the existing release binary while the workspace has
    # unrelated compile errors in other modules.
    # run_cmd(
    #     ["cargo", "build", "--release", "-p", "cortex-engine", "--bin", "enterprise_rag_bench_retrieval"],
    #     label="build retrieval binary",
    #     artifacts={"retrieval_binary": ROOT / "target/release/enterprise_rag_bench_retrieval"},
    # )
    cmd = [
        "./target/release/enterprise_rag_bench_retrieval",
        "--questions",
        str(p["clean_questions"]),
        "--uuid-index",
        str(args.uuid_index),
        "--sources-dir",
        str(args.sources_dir),
        "--db-root",
        str(p["db_root"]),
        "--output",
        str(p["raw_retrieval"]),
        "--report",
        str(p["retrieval_report"]),
        "--top-k",
        str(retrieval_top_k),
        "--batch-size",
        str(args.batch_size),
        "--progress-every",
        str(args.retrieval_progress_every),
        "--retrieval-mode",
        args.retrieval_mode,
        "--rerank",
        args.rerank,
        "--official-clean",
        "--log-file",
        str(p["retrieval_log"]),
        "--status-file",
        str(p["retrieval_status"]),
    ]
    if args.max_documents:
        cmd.extend(["--max-documents", str(args.max_documents)])
    if args.query_vectors:
        cmd.extend(["--query-vectors", str(args.query_vectors)])
    if args.document_vectors:
        cmd.extend(["--document-vectors", str(args.document_vectors)])
    if args.prefilter_retrieval:
        cmd.extend(["--prefilter-retrieval", str(args.prefilter_retrieval)])
    if args.disable_search_prefilter:
        cmd.append("--disable-search-prefilter")
    if args.reuse_db:
        cmd.append("--skip-ingest")
    else:
        cmd.append("--reset-db")
    if args.skip_checkpoint:
        cmd.append("--skip-checkpoint")
    run_cmd(
        cmd,
        label="retrieve with CortexDB",
        child_status=p["retrieval_status"],
        artifacts={
            "raw_retrieval": p["raw_retrieval"],
            "retrieval_report": p["retrieval_report"],
            "retrieval_log": p["retrieval_log"],
            "retrieval_status": p["retrieval_status"],
            "db_root": p["db_root"],
        },
    )
    run_cmd(
        [
            "python3",
            "scripts/enterprise_rag_bench/prepare_official_clean_inputs.py",
            "--questions-file",
            str(p["clean_questions"]),
            "--output-questions",
            str(p["clean_questions"]),
            "--report",
            str(p["retrieval_clean_report"]),
            "--retrieval-file",
            str(p["raw_retrieval"]),
            "--output-retrieval",
            str(sanitize_target),
            "--log-file",
            str(p["sanitize_log"]),
            "--status-file",
            str(p["sanitize_status"]),
        ],
        label="sanitize retrieval output",
        child_status=p["sanitize_status"],
        artifacts={
            "clean_retrieval": sanitize_target,
            "retrieval_clean_report": p["retrieval_clean_report"],
            "sanitize_log": p["sanitize_log"],
        },
    )
    if args.embedding_rerank:
        embedding_rerank(args, p)


def embedding_rerank(args: argparse.Namespace, p: dict[str, Path]) -> None:
    """Rerank the wide clean candidate set with an external embedding model.

    This is the EPIC-04 external-model reranker. It reads only the question text
    and candidate document bodies (no oracle labels), embeds them, and keeps the
    cosine-nearest `--top-k` documents. The output keeps the clean row shape, so
    the answer-stage guard still passes.
    """
    log(
        "embedding-rerank config "
        f"candidates={args.embedding_rerank_candidates} final_top_k={args.top_k} "
        f"env_file={args.env_file} cache={p['embedding_cache']}"
    )
    run_cmd(
        [
            "python3",
            "scripts/enterprise_rag_bench/rerank_with_embeddings.py",
            "--retrieval-file",
            str(p["clean_retrieval_wide"]),
            "--uuid-index",
            str(args.uuid_index),
            "--sources-dir",
            str(args.sources_dir),
            "--output",
            str(p["clean_retrieval"]),
            "--report",
            str(p["embedding_rerank_report"]),
            "--cache-file",
            str(p["embedding_cache"]),
            "--env-file",
            str(args.env_file),
            "--top-k",
            str(args.top_k),
            "--max-chars-per-doc",
            str(args.max_chars_per_doc),
        ],
        label="embedding rerank wide candidates",
        artifacts={
            "clean_retrieval": p["clean_retrieval"],
            "embedding_rerank_report": p["embedding_rerank_report"],
            "embedding_rerank_log": p["embedding_rerank_log"],
        },
    )
