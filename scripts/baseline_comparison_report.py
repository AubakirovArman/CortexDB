"""Report assembly for the C20 baseline comparison gate."""

from __future__ import annotations

import argparse
from typing import Any

from baseline_comparison_common import load_json, q16_pct
from baseline_comparison_index import dataset_report


def validate_features(features: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if features.get("schema_version") != "cortexdb.baseline_comparison.features.v1":
        failures.append("feature matrix schema_version mismatch")
    rows = features.get("rows")
    if not isinstance(rows, list) or not rows:
        failures.append("feature matrix rows must be non-empty")
        return failures
    for index, row in enumerate(rows, start=1):
        if not isinstance(row, dict):
            failures.append(f"feature row {index}: expected object")
            continue
        for field in ("feature", "naive_stack", "cortexdb", "evidence"):
            if not isinstance(row.get(field), str) or not row[field].strip():
                failures.append(f"feature row {index}: missing {field}")
    return failures


def cortexdb_domain_map(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    domains = report.get("domains")
    if not isinstance(domains, list):
        return {}
    return {str(row.get("domain")): row for row in domains if isinstance(row, dict)}


def build_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# C20 Baseline Comparison",
        "",
        "This is an honest local comparison against a small naive stack: SQLite FTS5, deterministic exact hashed vectors, and hybrid RRF. It also lists CortexDB feature evidence that the naive stack does not provide by itself.",
        "",
        "## Command",
        "",
        "```bash",
        "make baseline-comparison-check",
        "```",
        "",
        "## Retrieval And Latency",
        "",
        "| Domain | SQLite FTS5 recall | Vector recall | Hybrid recall | CortexDB gate recall | Hybrid p95 ms | CortexDB p95 ms |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for row in report["comparison"]:
        lines.append(
            "| {domain} | {fts} | {vec} | {hybrid} | {cortex} | {hybrid_ms:.3f} | {cortex_ms:.3f} |".format(
                domain=row["domain"],
                fts=q16_pct(row["sqlite_fts5_mean_hit_recall_q16"]),
                vec=q16_pct(row["hash_vector_mean_hit_recall_q16"]),
                hybrid=q16_pct(row["naive_hybrid_mean_hit_recall_q16"]),
                cortex=q16_pct(row["cortexdb_mean_hit_recall_q16"]),
                hybrid_ms=row["naive_hybrid_p95_latency_nanos"] / 1_000_000,
                cortex_ms=row["cortexdb_p95_latency_nanos"] / 1_000_000,
            )
        )
    context = report["cortexdb_context_pack"]
    lines.extend([
        "",
        "## Feature Matrix",
        "",
        "| Feature | Naive stack | CortexDB | Evidence |",
        "|---|---|---|---|",
    ])
    for row in report["feature_matrix"]["rows"]:
        lines.append(
            f"| {row['feature']} | {row['naive_stack']} | {row['cortexdb']} | {row['evidence']} |"
        )
    lines.extend([
        "",
        "## CortexDB ContextPack Evidence",
        "",
        f"- status: `{context.get('status', 'unknown')}`",
        f"- external datasets: `{context.get('external_dataset_count', 0)}`",
        f"- cases: `{context.get('case_count', 0)}`",
        f"- evidence coverage: `{q16_pct(int(context.get('evidence_coverage_q16', 0)))}`",
        f"- citation coverage: `{q16_pct(int(context.get('citation_coverage_q16', 0)))}`",
        f"- token reduction: `{q16_pct(int(context.get('token_reduction_q16', 0)))}`",
        "",
        "## Boundary",
        "",
        "- This report does not claim CortexDB always beats a naive retriever on raw recall.",
        "- The dense baseline is deterministic exact hashed-vector search so the gate stays dependency-free; it is the CI-safe stand-in for a FAISS sidecar.",
        "- The CortexDB differentiation shown here is retrieval plus built-in governance: permissions, token budgets, citations/provenance, and VerifyFact/conflict gates.",
        "",
    ])
    return "\n".join(lines)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = args.repo_root.resolve()
    datasets = load_json(args.datasets)
    feature_matrix = load_json(args.features)
    cortexdb_retrieval = load_json(args.cortexdb_retrieval_report)
    cortexdb_context = load_json(args.context_pack_report)
    failures = validate_features(feature_matrix)
    dataset_rows = datasets.get("datasets")
    if not isinstance(dataset_rows, list) or len(dataset_rows) < args.min_domains:
        failures.append(f"expected at least {args.min_domains} datasets")
        dataset_rows = []
    naive_domains = [
        dataset_report(root, row, repeat_runs=args.repeat_runs, top_k=args.top_k)
        for row in dataset_rows
        if isinstance(row, dict)
    ]
    cortex_domains = cortexdb_domain_map(cortexdb_retrieval)
    comparison = []
    for domain in naive_domains:
        comparison.append(domain_comparison_row(domain, cortex_domains, failures))
    if len(naive_domains) < args.min_domains:
        failures.append(f"baseline produced {len(naive_domains)} domains, expected {args.min_domains}")
    if cortexdb_context.get("status") != "passed":
        failures.append("ContextPack v3 evidence is not passed")
    return {
        "schema_version": "cortexdb.baseline_comparison.report.v1",
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "top_k": args.top_k,
        "repeat_runs": args.repeat_runs,
        "domain_count": len(naive_domains),
        "naive_stack": {
            "lexical": "sqlite_fts5",
            "dense": "deterministic_exact_hashed_vectors",
            "hybrid": "rrf(sqlite_fts5, deterministic_exact_hashed_vectors)",
            "dependency_policy": "stdlib only; no FAISS dependency is required by this gate",
        },
        "cortexdb_retrieval_source": str(args.cortexdb_retrieval_report),
        "cortexdb_context_pack_source": str(args.context_pack_report),
        "comparison": comparison,
        "naive_domains": naive_domains,
        "feature_matrix": feature_matrix,
        "cortexdb_context_pack": context_summary(cortexdb_context),
        "boundary": {
            "proves": "same-fixture naive retrieval baseline plus CortexDB retrieval/context-pack feature evidence",
            "does_not_prove": "raw recall superiority on every corpus or hosted FAISS/BGE embedding performance",
        },
    }


def domain_comparison_row(
    domain: dict[str, Any],
    cortex_domains: dict[str, dict[str, Any]],
    failures: list[str],
) -> dict[str, Any]:
    name = domain["domain"]
    hybrid = domain["strategies"]["naive_hybrid_rrf"]
    fts = domain["strategies"]["sqlite_fts5"]
    vector = domain["strategies"]["hash_vector"]
    cortex = cortex_domains.get(name, {})
    cortex_recall = int(cortex.get("latest_mean_recall_q16", 0))
    cortex_latency = int(cortex.get("latest_p95_latency_nanos", 0))
    if cortex_recall <= 0:
        failures.append(f"{name}: missing CortexDB retrieval gate evidence")
    return {
        "domain": name,
        "sqlite_fts5_mean_hit_recall_q16": fts["mean_hit_recall_q16"],
        "hash_vector_mean_hit_recall_q16": vector["mean_hit_recall_q16"],
        "naive_hybrid_mean_hit_recall_q16": hybrid["mean_hit_recall_q16"],
        "cortexdb_mean_hit_recall_q16": cortex_recall,
        "quality_delta_cortexdb_minus_hybrid_q16": cortex_recall - hybrid["mean_hit_recall_q16"],
        "naive_hybrid_p95_latency_nanos": hybrid["p95_latency_nanos"],
        "cortexdb_p95_latency_nanos": cortex_latency,
        "latency_delta_cortexdb_minus_hybrid_nanos": cortex_latency - hybrid["p95_latency_nanos"],
    }


def context_summary(report: dict[str, Any]) -> dict[str, Any]:
    return {
        key: report.get(key)
        for key in (
            "status",
            "case_count",
            "external_dataset_count",
            "failure_category_count",
            "evidence_coverage_q16",
            "citation_coverage_q16",
            "token_reduction_q16",
            "redundancy_reduction_q16",
            "anomaly_coverage_q16",
            "deterministic_order_q16",
        )
    }
