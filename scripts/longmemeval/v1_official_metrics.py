#!/usr/bin/env python3
"""Official-shape LongMemEval v1 retrieval metric helpers."""

from __future__ import annotations

import math
from typing import Any


KS = (1, 3, 5, 10, 30, 50)


def dcg(relevances: list[int], k: int) -> float:
    selected = relevances[:k]
    if not selected:
        return 0.0
    total = float(selected[0])
    for offset, value in enumerate(selected[1:], start=2):
        total += float(value) / math.log2(offset)
    return total


def ndcg(rankings: list[int], correct_docs: set[str], corpus_ids: list[str], k: int) -> float:
    relevances = [1 if corpus_id in correct_docs else 0 for corpus_id in corpus_ids]
    sorted_relevances = [relevances[index] for index in rankings[:k] if index < len(relevances)]
    ideal = sorted(relevances, reverse=True)
    ideal_dcg = dcg(ideal, k)
    if ideal_dcg == 0:
        return 0.0
    return dcg(sorted_relevances, k) / ideal_dcg


def evaluate_retrieval(
    rankings: list[int],
    correct_docs: set[str],
    corpus_ids: list[str],
    k: int,
) -> tuple[float, float, float]:
    recalled = {corpus_ids[index] for index in rankings[:k] if index < len(corpus_ids)}
    recall_any = float(any(doc in recalled for doc in correct_docs))
    recall_all = float(all(doc in recalled for doc in correct_docs))
    return recall_any, recall_all, ndcg(rankings, correct_docs, corpus_ids, k)


def strip_turn_id(doc_id: str) -> str:
    return "_".join(doc_id.split("_")[:-1])


def evaluate_turn2session(
    rankings: list[int],
    correct_docs: set[str],
    corpus_ids: list[str],
    k: int,
) -> tuple[float, float, float]:
    session_correct = {strip_turn_id(doc_id) for doc_id in correct_docs}
    session_corpus_ids = [strip_turn_id(doc_id) for doc_id in corpus_ids]
    effective_k = k
    unique_docids = {session_corpus_ids[index] for index in rankings[:effective_k]}
    while effective_k <= len(session_corpus_ids) and len(unique_docids) < k:
        effective_k += 1
        unique_docids = {session_corpus_ids[index] for index in rankings[:effective_k]}
    return evaluate_retrieval(rankings, session_correct, session_corpus_ids, effective_k)


def evaluate_entry(
    entry: dict[str, Any],
    corpus: list[dict[str, Any]],
    ranked_corpus_ids: list[str],
    granularity: str,
) -> dict[str, dict[str, float]]:
    corpus_ids = [row["corpus_id"] for row in corpus]
    id_to_index = {corpus_id: index for index, corpus_id in enumerate(corpus_ids)}
    rankings = [id_to_index[corpus_id] for corpus_id in ranked_corpus_ids if corpus_id in id_to_index]
    correct_docs = {corpus_id for corpus_id in corpus_ids if "answer" in corpus_id}
    metrics: dict[str, dict[str, float]] = {"session": {}, "turn": {}}
    for k in KS:
        recall_any, recall_all, ndcg_any = evaluate_retrieval(rankings, correct_docs, corpus_ids, k)
        metrics[granularity][f"recall_any@{k}"] = recall_any
        metrics[granularity][f"recall_all@{k}"] = recall_all
        metrics[granularity][f"ndcg_any@{k}"] = ndcg_any
        if granularity == "turn":
            recall_any, recall_all, ndcg_any = evaluate_turn2session(
                rankings, correct_docs, corpus_ids, k
            )
            metrics["session"][f"recall_any@{k}"] = recall_any
            metrics["session"][f"recall_all@{k}"] = recall_all
            metrics["session"][f"ndcg_any@{k}"] = ndcg_any
    return metrics


def should_skip_for_aggregate(entry: dict[str, Any], corpus: list[dict[str, Any]]) -> tuple[bool, str]:
    if "_abs" in str(entry["question_id"]):
        return True, "abstention"
    if not any("answer" in row["corpus_id"] for row in corpus):
        return True, "no_target"
    return False, ""


def summarize(results: list[dict[str, Any]], granularity: str) -> dict[str, Any]:
    aggregate: dict[str, float] = {}
    counted = [row for row in results if not row.get("aggregate_skip_reason")]
    metric_names = [f"{name}@{k}" for k in KS for name in ["recall_any", "recall_all", "ndcg_any"]]
    for name in metric_names:
        values = [
            row["retrieval_results"]["metrics"][granularity][name]
            for row in counted
            if name in row["retrieval_results"]["metrics"][granularity]
        ]
        aggregate[name] = sum(values) / len(values) if values else 0.0
    return {
        "question_count": len(results),
        "aggregate_count": len(counted),
        "skipped_abstention": sum(
            1 for row in results if row.get("aggregate_skip_reason") == "abstention"
        ),
        "skipped_no_target": sum(
            1 for row in results if row.get("aggregate_skip_reason") == "no_target"
        ),
        "granularity": granularity,
        "metrics": aggregate,
    }
