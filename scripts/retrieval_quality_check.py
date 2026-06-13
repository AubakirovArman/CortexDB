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


def marker_present(path: Path, marker: str) -> bool:
    return marker in path.read_text(encoding="utf-8")


def latest_run_report(history: dict[str, Any], latest: dict[str, Any]) -> dict[str, Any]:
    latest_run_id = latest.get("latest_run_id")
    runs = history.get("runs")
    if not isinstance(runs, list) or not latest_run_id:
        return {}
    for run in runs:
        if not isinstance(run, dict) or run.get("run_id") != latest_run_id:
            continue
        report_path = run.get("report")
        if isinstance(report_path, str) and report_path:
            path = Path(report_path)
            if path.is_file():
                return load_json(path)
    return {}


def query_level_rows(report: dict[str, Any]) -> list[dict[str, Any]]:
    rows = report.get("queries")
    if not isinstance(rows, list):
        return []
    output = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        output.append(
            {
                "name": row.get("name", ""),
                "recall_q16": row.get("recall_q16"),
                "mrr_q16": row.get("reciprocal_rank_q16"),
                "ndcg_q16": row.get("ndcg_q16"),
                "exact_parity": row.get("exact_parity"),
                "latency_nanos": row.get("latency_nanos"),
                "production_safe": row.get("production_safe"),
            }
        )
    return output


def mode_report(latest: dict[str, Any], latest_report: dict[str, Any]) -> dict[str, Any]:
    search_quality_tests = Path("crates/cortex-engine/tests/search_quality.rs")
    query_search_api_tests = Path("crates/cortex-engine/tests/query_search/api.rs")
    query_search_indexes_tests = Path("crates/cortex-engine/tests/query_search/indexes.rs")
    return {
        "lexical": {
            "status": "covered",
            "evidence": str(search_quality_tests),
            "gate": "cargo test -p cortex-engine --test search_quality",
            "bm25_golden": marker_present(
                search_quality_tests,
                "bm25_quality_fixture_has_perfect_mrr_for_golden_queries",
            ),
        },
        "vector": {
            "status": "covered",
            "evidence": str(query_search_api_tests),
            "gate": "cargo test -p cortex-engine --test query_search",
            "exact_vector_mode": marker_present(
                query_search_api_tests,
                "search_api_supports_keyword_and_vector_modes",
            ),
        },
        "hybrid": {
            "status": "covered",
            "evidence": str(query_search_indexes_tests),
            "gate": "cargo test -p cortex-engine --test query_search",
            "rrf_fusion": marker_present(query_search_indexes_tests, "hybrid_search_fuses_keyword"),
        },
        "guarded_ann": {
            "status": "measured",
            "run_id": latest.get("latest_run_id"),
            "mean_recall_q16": latest.get("latest_mean_recall_q16"),
            "mean_mrr_q16": latest.get("latest_mean_mrr_q16"),
            "mean_ndcg_q16": latest.get("latest_mean_ndcg_q16"),
            "exact_parity_q16": latest.get("latest_exact_parity_q16"),
            "p95_latency_nanos": latest.get("latest_p95_latency_nanos"),
            "production_safe": latest.get("latest_production_safe"),
            "query_level_rows": len(query_level_rows(latest_report)),
        },
    }


def validate(args: argparse.Namespace) -> dict[str, Any]:
    source_root = Path(args.source_root)
    docs_count = count_jsonl(source_root / "documents.jsonl")
    chunks_count = count_jsonl(source_root / "chunks.jsonl")
    query_count = count_jsonl(Path(args.queries))
    ground_truth_count = count_jsonl(Path(args.ground_truth))
    history = load_json(Path(args.history))
    latest = latest_corpus(history)
    ann_report = latest_run_report(history, latest)
    query_rows = query_level_rows(ann_report)
    modes = mode_report(latest, ann_report)

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
    for mode, report in modes.items():
        if report.get("status") not in {"covered", "measured"}:
            failures.append(f"{mode} mode is not covered")
    if not modes["lexical"].get("bm25_golden"):
        failures.append("lexical mode lacks BM25 golden evidence")
    if not modes["vector"].get("exact_vector_mode"):
        failures.append("vector mode lacks exact vector evidence")
    if not modes["hybrid"].get("rrf_fusion"):
        failures.append("hybrid mode lacks RRF fusion evidence")
    if not query_rows:
        failures.append("latest guarded ANN report lacks query-level rows")
    failures.extend(
        require_terms(
            Path(args.benchmarks),
            [
                "Real-Domain Embedding Baseline: Investment Projects",
                "mean_recall_q16",
                "mean_mrr_q16",
                "mean_ndcg_q16",
                "exact_parity_q16",
                "lexical",
                "vector",
                "hybrid",
                "guarded ANN",
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
        "modes": modes,
        "query_level": query_rows,
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
    parser.add_argument("--min-history-runs", type=int, default=3)
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
