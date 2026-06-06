#!/usr/bin/env python3
"""Validate the public retrieval benchmark page against local reports."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def require(text: str, marker: str, failures: list[str], path: Path) -> None:
    if marker not in text:
        failures.append(f"{path}: missing {marker!r}")


def history_by_domain(history: dict[str, Any]) -> dict[str, dict[str, Any]]:
    domains = history.get("domains")
    if not isinstance(domains, list):
        raise ValueError("history report missing domains list")
    return {str(row.get("domain")): row for row in domains if isinstance(row, dict)}


def validate(args: argparse.Namespace) -> dict[str, Any]:
    page = args.page.read_text(encoding="utf-8")
    beta = load_json(args.beta_report)
    history = load_json(args.history_report)
    failures: list[str] = []

    if beta.get("status") != "passed":
        failures.append("beta report is not passed")
    if history.get("status") != "passed":
        failures.append("history report is not passed")
    if history.get("regression_count") != 0:
        failures.append("history report has regressions")

    for marker in [
        "## Dataset Size",
        "## Latest Local Metrics",
        "## Exact Vs ANN",
        "## Limitations",
        "p95 latency",
        "p99 latency",
        "fallback-free production HNSW",
        "quality on private customer corpora",
        "make public-retrieval-benchmark-page-check",
    ]:
        require(page, marker, failures, args.page)

    history_domains = history_by_domain(history)
    totals = {"documents": 0, "chunks": 0, "queries": 0, "ground_truth": 0}
    for row in beta.get("domains", []):
        if not isinstance(row, dict):
            failures.append("beta report contains non-object domain row")
            continue
        domain = str(row.get("domain", ""))
        hrow = history_domains.get(domain)
        if not hrow:
            failures.append(f"history report missing domain {domain}")
            continue
        require(page, f"`{domain}`", failures, args.page)
        for field in totals:
            value = int(row.get(field, 0))
            totals[field] += value
            require(page, str(value), failures, args.page)
        for field in [
            "latest_mean_recall_q16",
            "latest_mean_mrr_q16",
            "latest_mean_ndcg_q16",
        ]:
            require(page, str(hrow.get(field)), failures, args.page)
        if int(hrow.get("history_runs", 0)) < args.min_runs:
            failures.append(f"{domain}: history runs below {args.min_runs}")

    for value in totals.values():
        require(page, str(value), failures, args.page)
    if int(history.get("run_count", 0)) < args.min_total_runs:
        failures.append(f"history run_count below {args.min_total_runs}")

    return {
        "schema_version": "cortexdb.public_retrieval_benchmarks.report.v1",
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "page": str(args.page),
        "beta_report": str(args.beta_report),
        "history_report": str(args.history_report),
        "domain_count": beta.get("domain_count"),
        "run_count": history.get("run_count"),
        "regression_count": history.get("regression_count"),
        "totals": totals,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--page", type=Path, default=Path("docs/PUBLIC_RETRIEVAL_BENCHMARKS.md"))
    parser.add_argument("--beta-report", type=Path, required=True)
    parser.add_argument("--history-report", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--min-runs", type=int, default=5)
    parser.add_argument("--min-total-runs", type=int, default=20)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(args)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"public retrieval benchmark check failed: {error}", file=sys.stderr)
        return 1
    output = args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"public retrieval benchmark check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
