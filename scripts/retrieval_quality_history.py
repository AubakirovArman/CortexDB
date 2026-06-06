#!/usr/bin/env python3
"""Build a repeated multi-domain retrieval quality history report."""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
import time
from pathlib import Path
from typing import Any

Q16_ONE = 65_535


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number}: expected object")
            rows.append(value)
    return rows


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def mean(values: list[int]) -> int:
    return min(Q16_ONE, sum(values) // len(values)) if values else Q16_ONE


def percentile(values: list[int], pct: float) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, math.ceil((pct / 100.0) * len(ordered)) - 1))
    return ordered[index]


def tokens(text: str) -> set[str]:
    return {
        token
        for token in re.split(r"[^0-9A-Za-zА-Яа-яЁёІіҢңҒғҮүҰұҚқӨөҺһ]+", text.lower())
        if token
    }


def query_id(row: dict[str, Any]) -> str:
    for key in ["query_id", "name"]:
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError("query row missing query_id/name")


def query_text(row: dict[str, Any]) -> str:
    for key in ["query", "text", "name"]:
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError(f"{query_id(row)}: missing query text")


def reciprocal_rank(top: list[str], relevant: set[str]) -> int:
    if not relevant:
        return Q16_ONE
    for rank, chunk_id in enumerate(top, start=1):
        if chunk_id in relevant:
            return max(1, Q16_ONE // rank)
    return 0


def ndcg(top: list[str], relevant: set[str]) -> int:
    if not relevant:
        return Q16_ONE
    gains = [
        1.0 / math.log2(rank + 1)
        for rank, chunk_id in enumerate(top, start=1)
        if chunk_id in relevant
    ]
    ideal_count = min(len(relevant), len(top))
    if ideal_count == 0:
        return Q16_ONE
    ideal = sum(1.0 / math.log2(rank + 1) for rank in range(1, ideal_count + 1))
    return min(Q16_ONE, int((sum(gains) / ideal) * Q16_ONE)) if ideal > 0.0 else 0


def domain_paths(domain: Path) -> dict[str, Path]:
    return {
        "chunks": domain / "corpus" / "chunks.jsonl",
        "queries": domain / "queries" / "queries.jsonl",
        "ground_truth": domain / "queries" / "ground_truth.jsonl",
    }


def discover_domains(root: Path) -> list[Path]:
    domains = []
    for child in sorted(root.iterdir()):
        if child.is_dir() and all(path.is_file() for path in domain_paths(child).values()):
            domains.append(child)
    return domains


def load_truth(rows: list[dict[str, Any]]) -> dict[str, set[str]]:
    return {
        str(row.get("query_id", "")): {str(value) for value in row.get("relevant_chunk_ids", [])}
        for row in rows
    }


def evaluate_domain(domain: Path, *, run_index: int, top_k: int) -> dict[str, Any]:
    paths = domain_paths(domain)
    chunks = load_jsonl(paths["chunks"])
    queries = load_jsonl(paths["queries"])
    truth = load_truth(load_jsonl(paths["ground_truth"]))
    chunk_terms = [
        (str(chunk["chunk_id"]), tokens(str(chunk.get("text", "")))) for chunk in chunks
    ]
    hit_count = 0
    mrr_values: list[int] = []
    ndcg_values: list[int] = []
    latency_values: list[int] = []
    first_top_by_query: dict[str, list[str]] = {}

    for query in queries:
        started = time.perf_counter_ns()
        qid = query_id(query)
        qterms = tokens(query_text(query))
        scored = [(len(qterms.intersection(cterms)), cid) for cid, cterms in chunk_terms]
        scored.sort(key=lambda item: (-item[0], item[1]))
        top = [cid for score, cid in scored[:top_k] if score > 0]
        latency = max(1, time.perf_counter_ns() - started)
        relevant = truth.get(qid, set())
        hit = bool(relevant.intersection(top))
        mrr = reciprocal_rank(top, relevant)
        ndcg_value = ndcg(top, relevant)
        hit_count += int(hit)
        mrr_values.append(mrr)
        ndcg_values.append(ndcg_value)
        latency_values.append(latency)
        first_top_by_query[qid] = top

    return {
        "run_id": f"{domain.name}-history-{run_index:03d}",
        "domain": domain.name,
        "run_index": run_index,
        "query_count": len(queries),
        "chunk_count": len(chunks),
        "hit_count": hit_count,
        "mean_recall_q16": q16(hit_count, len(queries)),
        "mean_mrr_q16": mean(mrr_values),
        "mean_ndcg_q16": mean(ndcg_values),
        "p50_latency_nanos": percentile(latency_values, 50),
        "p95_latency_nanos": percentile(latency_values, 95),
        "p99_latency_nanos": percentile(latency_values, 99),
        "max_latency_nanos": max(latency_values) if latency_values else 0,
        "top_k": top_k,
        "top_by_query": first_top_by_query,
    }


def regression(kind: str, field: str, prev: dict[str, Any], cur: dict[str, Any]) -> dict[str, Any]:
    return {
        "kind": kind,
        "field": field,
        "domain": cur["domain"],
        "previous_run_id": prev["run_id"],
        "current_run_id": cur["run_id"],
        "previous": prev[field],
        "current": cur[field],
        "delta": int(cur[field]) - int(prev[field]),
    }


def compare(prev: dict[str, Any], cur: dict[str, Any], args: argparse.Namespace) -> list[dict[str, Any]]:
    regressions = []
    for field in ["mean_recall_q16", "mean_mrr_q16", "mean_ndcg_q16"]:
        if int(cur[field]) < int(prev[field]):
            regressions.append(regression("quality", field, prev, cur))
    limits = {
        "p95_latency_nanos": args.max_p95_regression_nanos,
        "p99_latency_nanos": args.max_p99_regression_nanos,
        "max_latency_nanos": args.max_max_regression_nanos,
    }
    for field, allowed in limits.items():
        if int(cur[field]) - int(prev[field]) > allowed:
            regressions.append(regression("latency", field, prev, cur))
    if cur["top_by_query"] != prev["top_by_query"]:
        regressions.append(regression("exact_parity", "run_index", prev, cur))
    return regressions


def summarize_domain(domain: str, runs: list[dict[str, Any]], regressions: list[dict[str, Any]]) -> dict[str, Any]:
    latest = runs[-1]
    return {
        "domain": domain,
        "history_runs": len(runs),
        "latest_run_id": latest["run_id"],
        "latest_mean_recall_q16": latest["mean_recall_q16"],
        "latest_mean_mrr_q16": latest["mean_mrr_q16"],
        "latest_mean_ndcg_q16": latest["mean_ndcg_q16"],
        "latest_p95_latency_nanos": latest["p95_latency_nanos"],
        "latest_p99_latency_nanos": latest["p99_latency_nanos"],
        "latest_max_latency_nanos": latest["max_latency_nanos"],
        "regression_count": len(regressions),
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    domains = discover_domains(args.domain_root)
    failures: list[str] = []
    if len(domains) < args.min_domains:
        failures.append(f"expected {args.min_domains} domains, found {len(domains)}")
    all_runs: list[dict[str, Any]] = []
    all_regressions: list[dict[str, Any]] = []
    summaries = []
    for domain in domains:
        runs = [evaluate_domain(domain, run_index=i + 1, top_k=args.top_k) for i in range(args.history_runs)]
        regressions = []
        for prev, cur in zip(runs, runs[1:]):
            regressions.extend(compare(prev, cur, args))
        all_runs.extend(runs)
        all_regressions.extend(regressions)
        summaries.append(summarize_domain(domain.name, runs, regressions))
        latest = runs[-1]
        if latest["mean_recall_q16"] <= 0:
            failures.append(f"{domain.name}: recall must be positive")
    if args.fail_on_regression and all_regressions:
        failures.append(f"found {len(all_regressions)} retrieval quality regression(s)")
    return {
        "schema_version": "cortexdb.retrieval_quality_history.v1",
        "status": "passed" if not failures else "failed",
        "production_safe": not failures,
        "failures": failures,
        "domain_count": len(domains),
        "history_runs_per_domain": args.history_runs,
        "top_k": args.top_k,
        "run_count": len(all_runs),
        "regression_count": len(all_regressions),
        "domains": summaries,
        "regressions": all_regressions,
        "runs": all_runs,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--domain-root", type=Path, default=Path("examples/real_domains"))
    parser.add_argument("--output", type=Path, required=False)
    parser.add_argument("--min-domains", type=int, default=4)
    parser.add_argument("--history-runs", type=int, default=5)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--fail-on-regression", action="store_true")
    parser.add_argument("--max-p95-regression-nanos", type=int, default=50_000_000)
    parser.add_argument("--max-p99-regression-nanos", type=int, default=100_000_000)
    parser.add_argument("--max-max-regression-nanos", type=int, default=100_000_000)
    return parser.parse_args(argv)


def run(args: argparse.Namespace) -> int:
    report = build_report(args)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    if args.output:
        print(f"retrieval quality history passed: {args.output}")
    return 0

def main(argv: list[str]) -> int:
    return run(parse_args(argv))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
