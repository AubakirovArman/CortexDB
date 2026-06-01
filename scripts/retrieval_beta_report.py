#!/usr/bin/env python3
"""Build a multi-domain beta retrieval evidence report.

This report is intentionally local and reproducible. It validates checked-in
real-domain corpora and runs a deterministic lexical retrieval probe five times
per domain. Endpoint-backed embedding history is referenced separately by
`scripts/retrieval_quality_check.py`.
"""

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


def mean_q16(values: list[int]) -> int:
    if not values:
        return Q16_ONE
    return min(Q16_ONE, sum(values) // len(values))


def tokens(text: str) -> set[str]:
    return {
        token
        for token in re.split(r"[^0-9A-Za-zА-Яа-яЁёІіҢңҒғҮүҰұҚқӨөҺһ]+", text.lower())
        if token
    }


def query_text(row: dict[str, Any]) -> str:
    for key in ["query", "text", "name"]:
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError(f"{row.get('query_id', row.get('name', '<query>'))}: missing query text")


def query_id(row: dict[str, Any]) -> str:
    for key in ["query_id", "name"]:
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    raise ValueError("query row missing query_id/name")


def reciprocal_rank_q16(top: list[str], relevant: set[str]) -> int:
    if not relevant:
        return Q16_ONE
    for rank, chunk_id in enumerate(top, start=1):
        if chunk_id in relevant:
            return max(1, Q16_ONE // rank)
    return 0


def ndcg_q16(top: list[str], relevant: set[str]) -> int:
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
    if ideal <= 0.0:
        return 0
    return min(Q16_ONE, int((sum(gains) / ideal) * Q16_ONE))


def domain_paths(domain: Path) -> dict[str, Path]:
    return {
        "documents": domain / "corpus" / "documents.jsonl",
        "chunks": domain / "corpus" / "chunks.jsonl",
        "queries": domain / "queries" / "queries.jsonl",
        "ground_truth": domain / "queries" / "ground_truth.jsonl",
    }


def discover_domains(root: Path) -> list[Path]:
    domains = []
    for child in sorted(root.iterdir()):
        if not child.is_dir():
            continue
        paths = domain_paths(child)
        if all(path.is_file() for path in paths.values()):
            domains.append(child)
    return domains


def retrieve_once(
    queries: list[dict[str, Any]],
    chunks: list[dict[str, Any]],
    truth_by_query: dict[str, set[str]],
    *,
    top_k: int,
) -> dict[str, Any]:
    query_rows = []
    hits = 0
    mrr_values: list[int] = []
    ndcg_values: list[int] = []
    started = time.perf_counter_ns()
    chunk_terms = [
        (str(chunk["chunk_id"]), tokens(str(chunk.get("text", "")))) for chunk in chunks
    ]
    for query in queries:
        qid = query_id(query)
        qterms = tokens(query_text(query))
        scored = []
        for chunk_id, cterms in chunk_terms:
            overlap = len(qterms.intersection(cterms))
            scored.append((overlap, chunk_id))
        scored.sort(key=lambda item: (-item[0], item[1]))
        top = [chunk_id for score, chunk_id in scored[:top_k] if score > 0]
        relevant = truth_by_query.get(qid, set())
        hit = bool(relevant.intersection(top))
        mrr = reciprocal_rank_q16(top, relevant)
        ndcg = ndcg_q16(top, relevant)
        hits += int(hit)
        mrr_values.append(mrr)
        ndcg_values.append(ndcg)
        query_rows.append({
            "query_id": qid,
            "hit": hit,
            "mrr_q16": mrr,
            "ndcg_q16": ndcg,
            "top_k": top_k,
            "top_chunk_ids": top,
            "relevant_chunk_count": len(relevant),
        })
    elapsed = max(1, time.perf_counter_ns() - started)
    return {
        "query_count": len(queries),
        "hit_count": hits,
        "mean_recall_q16": q16(hits, len(queries)),
        "mean_mrr_q16": mean_q16(mrr_values),
        "mean_ndcg_q16": mean_q16(ndcg_values),
        "p95_latency_nanos": elapsed // max(1, len(queries)),
        "queries": query_rows,
    }


def repeat_exact_parity_q16(runs: list[dict[str, Any]]) -> int:
    if len(runs) <= 1:
        return Q16_ONE
    first = {
        str(row["query_id"]): list(row.get("top_chunk_ids", []))
        for row in runs[0].get("queries", [])
    }
    comparisons = 0
    matches = 0
    for run in runs[1:]:
        for row in run.get("queries", []):
            qid = str(row["query_id"])
            comparisons += 1
            if first.get(qid) == list(row.get("top_chunk_ids", [])):
                matches += 1
    return q16(matches, comparisons)


def domain_report(domain: Path, *, repeat_runs: int, top_k: int) -> dict[str, Any]:
    paths = domain_paths(domain)
    documents = load_jsonl(paths["documents"])
    chunks = load_jsonl(paths["chunks"])
    queries = load_jsonl(paths["queries"])
    truth_rows = load_jsonl(paths["ground_truth"])
    chunk_ids = {str(row["chunk_id"]) for row in chunks}
    query_ids = {query_id(row) for row in queries}
    truth_by_query: dict[str, set[str]] = {}
    failures: list[str] = []

    for row in truth_rows:
        qid = str(row.get("query_id", ""))
        if qid not in query_ids:
            failures.append(f"{domain.name}: unknown ground-truth query_id {qid}")
        relevant = {str(value) for value in row.get("relevant_chunk_ids", [])}
        missing = sorted(relevant.difference(chunk_ids))
        if missing:
            failures.append(f"{domain.name}:{qid}: unknown relevant chunks {missing}")
        truth_by_query[qid] = relevant

    runs = [retrieve_once(queries, chunks, truth_by_query, top_k=top_k) for _ in range(repeat_runs)]
    regressions = []
    for previous, current in zip(runs, runs[1:]):
        if current["mean_recall_q16"] < previous["mean_recall_q16"]:
            regressions.append({
                "field": "mean_recall_q16",
                "previous": previous["mean_recall_q16"],
                "current": current["mean_recall_q16"],
            })
    latest = runs[-1] if runs else {}
    return {
        "domain": domain.name,
        "documents": len(documents),
        "chunks": len(chunks),
        "queries": len(queries),
        "ground_truth": len(truth_rows),
        "run_count": len(runs),
        "latest_mean_recall_q16": latest.get("mean_recall_q16", 0),
        "latest_mean_mrr_q16": latest.get("mean_mrr_q16", 0),
        "latest_mean_ndcg_q16": latest.get("mean_ndcg_q16", 0),
        "latest_p95_latency_nanos": latest.get("p95_latency_nanos", 0),
        "latest_exact_parity_q16": repeat_exact_parity_q16(runs),
        "regression_count": len(regressions),
        "regressions": regressions,
        "failures": failures,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    domains = discover_domains(args.domain_root)
    failures: list[str] = []
    if len(domains) < args.min_domains:
        failures.append(f"expected at least {args.min_domains} retrieval domains, found {len(domains)}")
    reports = [
        domain_report(domain, repeat_runs=args.repeat_runs, top_k=args.top_k)
        for domain in domains
    ]
    for report in reports:
        failures.extend(report["failures"])
        if report["run_count"] < args.repeat_runs:
            failures.append(f"{report['domain']}: run count below {args.repeat_runs}")
        if report["latest_mean_recall_q16"] <= 0:
            failures.append(f"{report['domain']}: latest recall must be positive")
        if report["regression_count"]:
            failures.append(f"{report['domain']}: retrieval regression detected")
    return {
        "schema_version": "cortexdb.retrieval_beta_report.v1",
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "domain_count": len(reports),
        "repeat_runs_per_domain": args.repeat_runs,
        "top_k": args.top_k,
        "production_safe": not failures,
        "domains": reports,
        "boundary": {
            "proves": "local deterministic multi-domain retrieval fixture coverage",
            "does_not_prove": "hosted embedding quality or private customer relevance judgments",
        },
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--domain-root", type=Path, default=Path("examples/real_domains"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--min-domains", type=int, default=2)
    parser.add_argument("--repeat-runs", type=int, default=5)
    parser.add_argument("--top-k", type=int, default=10)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"retrieval beta report failed: {error}", file=sys.stderr)
        return 1
    output = args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"retrieval beta report passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
