"""Naive baseline indexes and metrics for C20."""

from __future__ import annotations

import sqlite3
import time
from pathlib import Path
from typing import Any

from baseline_comparison_common import (
    chunk_text,
    dot,
    fts_query,
    load_jsonl,
    mean_q16,
    ndcg_q16,
    p95,
    q16,
    query_id,
    query_text,
    reciprocal_rank_q16,
    repo_path,
    vectorize,
)


class BaselineIndex:
    def __init__(self, chunks: list[dict[str, Any]]) -> None:
        self.chunk_rows = []
        self.vectors: dict[str, list[float]] = {}
        self.conn = sqlite3.connect(":memory:")
        self._build_fts(chunks)
        for ordinal, chunk in enumerate(chunks):
            chunk_id = str(chunk["chunk_id"])
            text = chunk_text(chunk)
            self.chunk_rows.append((chunk_id, text, ordinal))
            self.vectors[chunk_id] = vectorize(text)

    def _build_fts(self, chunks: list[dict[str, Any]]) -> None:
        try:
            self.conn.execute(
                "CREATE VIRTUAL TABLE chunks_fts USING fts5("
                "chunk_id UNINDEXED, doc_id UNINDEXED, title, body, tokenize='unicode61')"
            )
        except sqlite3.OperationalError as error:
            raise RuntimeError("SQLite FTS5 is unavailable in this Python build") from error
        rows = [
            (
                str(chunk["chunk_id"]),
                str(chunk.get("doc_id", "")),
                str(chunk.get("title", "")),
                str(chunk.get("text") or chunk.get("payload") or ""),
            )
            for chunk in chunks
        ]
        self.conn.executemany(
            "INSERT INTO chunks_fts(chunk_id, doc_id, title, body) VALUES (?, ?, ?, ?)",
            rows,
        )

    def search_fts5(self, text: str, top_k: int) -> list[str]:
        expression = fts_query(text)
        if not expression:
            return []
        cursor = self.conn.execute(
            "SELECT chunk_id FROM chunks_fts "
            "WHERE chunks_fts MATCH ? "
            "ORDER BY bm25(chunks_fts), chunk_id "
            "LIMIT ?",
            (expression, top_k),
        )
        return [str(row[0]) for row in cursor.fetchall()]

    def search_vector(self, text: str, top_k: int) -> list[str]:
        query_vector = vectorize(text)
        scored = [
            (dot(query_vector, vector), ordinal, chunk_id)
            for chunk_id, _text, ordinal in self.chunk_rows
            for vector in [self.vectors[chunk_id]]
        ]
        scored.sort(key=lambda row: (-row[0], row[1], row[2]))
        return [chunk_id for score, _ordinal, chunk_id in scored[:top_k] if score > 0.0]

    def search_hybrid(self, text: str, top_k: int) -> list[str]:
        fts = self.search_fts5(text, top_k * 3)
        vec = self.search_vector(text, top_k * 3)
        scores: dict[str, float] = {}
        first_rank: dict[str, int] = {}
        for result_set in (fts, vec):
            for rank, chunk_id in enumerate(result_set, start=1):
                scores[chunk_id] = scores.get(chunk_id, 0.0) + 1.0 / (60.0 + rank)
                first_rank.setdefault(chunk_id, rank)
        ranked = sorted(scores, key=lambda chunk_id: (-scores[chunk_id], first_rank[chunk_id], chunk_id))
        return ranked[:top_k]


def evaluate(
    index: BaselineIndex,
    queries: list[dict[str, Any]],
    truth_by_query: dict[str, set[str]],
    *,
    strategy: str,
    repeat_runs: int,
    top_k: int,
) -> dict[str, Any]:
    searchers = {
        "sqlite_fts5": index.search_fts5,
        "hash_vector": index.search_vector,
        "naive_hybrid_rrf": index.search_hybrid,
    }
    search = searchers[strategy]
    latencies: list[int] = []
    latest_rows: list[dict[str, Any]] = []
    for run_index in range(repeat_runs):
        run_rows = []
        for query in queries:
            qid = query_id(query)
            started = time.perf_counter_ns()
            top = search(query_text(query), top_k)
            latencies.append(max(1, time.perf_counter_ns() - started))
            relevant = truth_by_query.get(qid, set())
            hits = len(relevant.intersection(top))
            run_rows.append({
                "query_id": qid,
                "hit": hits > 0,
                "hit_count": hits,
                "relevant_chunk_count": len(relevant),
                "recall_q16": q16(hits, len(relevant)),
                "mrr_q16": reciprocal_rank_q16(top, relevant),
                "ndcg_q16": ndcg_q16(top, relevant),
                "top_chunk_ids": top,
            })
        if run_index == repeat_runs - 1:
            latest_rows = run_rows
    return {
        "strategy": strategy,
        "query_count": len(queries),
        "hit_count": sum(1 for row in latest_rows if row["hit"]),
        "mean_hit_recall_q16": q16(sum(1 for row in latest_rows if row["hit"]), len(latest_rows)),
        "mean_relevant_recall_q16": mean_q16([row["recall_q16"] for row in latest_rows]),
        "mean_mrr_q16": mean_q16([row["mrr_q16"] for row in latest_rows]),
        "mean_ndcg_q16": mean_q16([row["ndcg_q16"] for row in latest_rows]),
        "p95_latency_nanos": p95(latencies),
        "queries": latest_rows,
    }


def dataset_report(root: Path, dataset: dict[str, Any], *, repeat_runs: int, top_k: int) -> dict[str, Any]:
    domain = str(dataset.get("domain") or dataset.get("dataset_id"))
    chunks = load_jsonl(repo_path(root, str(dataset["corpus"])))
    queries = load_jsonl(repo_path(root, str(dataset["queries"])))
    truth_rows = load_jsonl(repo_path(root, str(dataset["ground_truth"])))
    truth_by_query = {
        str(row["query_id"]): {str(value) for value in row.get("relevant_chunk_ids", [])}
        for row in truth_rows
    }
    started = time.perf_counter_ns()
    index = BaselineIndex(chunks)
    build_nanos = max(1, time.perf_counter_ns() - started)
    strategies = [
        evaluate(index, queries, truth_by_query, strategy=strategy, repeat_runs=repeat_runs, top_k=top_k)
        for strategy in ("sqlite_fts5", "hash_vector", "naive_hybrid_rrf")
    ]
    return {
        "domain": domain,
        "chunks": len(chunks),
        "queries": len(queries),
        "ground_truth": len(truth_rows),
        "index_build_nanos": build_nanos,
        "strategies": {row["strategy"]: row for row in strategies},
    }
