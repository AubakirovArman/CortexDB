#!/usr/bin/env python3
"""Validate local retrieval-quality evidence for the real-domain corpus."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def count_jsonl(path: Path) -> int:
    count = 0
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            try:
                json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {error}") from error
            count += 1
    return count


def require_terms(path: Path, terms: list[str]) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return [f"{path}: missing {term!r}" for term in terms if term not in text]


def latest_corpus(history: dict[str, Any]) -> dict[str, Any]:
    corpora = history.get("corpora")
    if not isinstance(corpora, list) or not corpora:
        raise ValueError("history: expected at least one corpus group")
    latest = corpora[-1]
    if not isinstance(latest, dict):
        raise ValueError("history: latest corpus must be an object")
    return latest


def int_field(value: dict[str, Any], field: str) -> int:
    raw = value.get(field)
    if not isinstance(raw, int):
        raise ValueError(f"history:{field}: expected integer")
    return raw


def validate(args: argparse.Namespace) -> dict[str, Any]:
    source_root = Path(args.source_root)
    docs_count = count_jsonl(source_root / "documents.jsonl")
    chunks_count = count_jsonl(source_root / "chunks.jsonl")
    query_count = count_jsonl(Path(args.queries))
    ground_truth_count = count_jsonl(Path(args.ground_truth))
    history = load_json(Path(args.history))
    latest = latest_corpus(history)

    failures: list[str] = []
    if docs_count < args.min_docs:
        failures.append(f"documents below minimum: {docs_count} < {args.min_docs}")
    if chunks_count < args.min_chunks:
        failures.append(f"chunks below minimum: {chunks_count} < {args.min_chunks}")
    if query_count < args.min_queries:
        failures.append(f"queries below minimum: {query_count} < {args.min_queries}")
    if ground_truth_count < args.min_queries:
        failures.append(f"ground truth below minimum: {ground_truth_count} < {args.min_queries}")
    if int_field(history, "run_count") < args.min_history_runs:
        failures.append(
            f"history runs below minimum: {history['run_count']} < {args.min_history_runs}"
        )
    if int_field(history, "regression_count") != 0:
        failures.append(f"history has regressions: {history['regression_count']}")
    if latest.get("latest_production_safe") is not True:
        failures.append("latest corpus is not production safe")
    for field in [
        "latest_mean_recall_q16",
        "latest_mean_mrr_q16",
        "latest_mean_ndcg_q16",
        "latest_exact_parity_q16",
    ]:
        if int_field(latest, field) <= 0:
            failures.append(f"latest corpus {field} must be > 0")
    failures.extend(
        require_terms(
            Path(args.benchmarks),
            [
                "Real-Domain Embedding Baseline: Investment Projects",
                "mean_recall_q16",
                "mean_mrr_q16",
                "mean_ndcg_q16",
                "exact_parity_q16",
            ],
        )
    )

    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "documents": docs_count,
        "chunks": chunks_count,
        "queries": query_count,
        "ground_truth": ground_truth_count,
        "history": {
            "run_count": history["run_count"],
            "corpus_count": history.get("corpus_count"),
            "regression_count": history["regression_count"],
            "latest_run_id": latest.get("latest_run_id"),
            "latest_mean_recall_q16": latest.get("latest_mean_recall_q16"),
            "latest_mean_mrr_q16": latest.get("latest_mean_mrr_q16"),
            "latest_mean_ndcg_q16": latest.get("latest_mean_ndcg_q16"),
            "latest_exact_parity_q16": latest.get("latest_exact_parity_q16"),
            "latest_p95_latency_nanos": latest.get("latest_p95_latency_nanos"),
            "latest_production_safe": latest.get("latest_production_safe"),
        },
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", required=True)
    parser.add_argument("--queries", required=True)
    parser.add_argument("--ground-truth", required=True)
    parser.add_argument("--history", required=True)
    parser.add_argument("--benchmarks", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--min-docs", type=int, default=50)
    parser.add_argument("--min-chunks", type=int, default=150)
    parser.add_argument("--min-queries", type=int, default=40)
    parser.add_argument("--min-history-runs", type=int, default=2)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(args)
    except (OSError, ValueError) as error:
        print(f"retrieval quality check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"retrieval quality check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
