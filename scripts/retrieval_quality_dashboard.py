#!/usr/bin/env python3
"""Render a static retrieval-quality dashboard from local evidence reports."""

from __future__ import annotations

import argparse
import html
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535
REQUIRED_DOMAIN_FIELDS = [
    "domain",
    "documents",
    "chunks",
    "queries",
    "latest_mean_recall_q16",
    "latest_mean_mrr_q16",
    "latest_mean_ndcg_q16",
    "latest_p95_latency_nanos",
    "latest_exact_parity_q16",
]


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def q16_percent(value: Any) -> str:
    if not isinstance(value, int):
        return "n/a"
    return f"{(value * 100) / Q16_ONE:.2f}%"


def millis(value: Any) -> str:
    if not isinstance(value, int):
        return "n/a"
    return f"{value / 1_000_000:.3f} ms"


def text(value: Any) -> str:
    return html.escape(str(value))


def metric(label: str, value: Any, *, tone: str = "") -> str:
    tone_attr = f' class="{tone}"' if tone else ""
    return f"<li{tone_attr}><span>{text(label)}</span><strong>{text(value)}</strong></li>"


def row(cells: list[Any]) -> str:
    return "<tr>" + "".join(f"<td>{text(cell)}</td>" for cell in cells) + "</tr>"


def validate_beta_report(beta: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    domains = beta.get("domains")
    if not isinstance(domains, list) or not domains:
        return ["beta report has no domains"]
    for domain in domains:
        if not isinstance(domain, dict):
            failures.append("domain row is not an object")
            continue
        name = domain.get("domain", "<unknown>")
        for field in REQUIRED_DOMAIN_FIELDS:
            if field not in domain:
                failures.append(f"{name}: missing {field}")
    return failures


def guarded_ann_metrics(report: dict[str, Any]) -> dict[str, Any]:
    modes = report.get("modes", {})
    if not isinstance(modes, dict):
        return {}
    guarded = modes.get("guarded_ann", {})
    return guarded if isinstance(guarded, dict) else {}


def render_domain_table(domains: list[dict[str, Any]]) -> str:
    headers = [
        "Domain",
        "Docs",
        "Chunks",
        "Queries",
        "Recall",
        "MRR",
        "nDCG",
        "p95 latency",
        "Exact parity",
        "Regressions",
    ]
    body = "\n".join(
        row([
            domain.get("domain", "unknown"),
            domain.get("documents", 0),
            domain.get("chunks", 0),
            domain.get("queries", 0),
            q16_percent(domain.get("latest_mean_recall_q16")),
            q16_percent(domain.get("latest_mean_mrr_q16")),
            q16_percent(domain.get("latest_mean_ndcg_q16")),
            millis(domain.get("latest_p95_latency_nanos")),
            q16_percent(domain.get("latest_exact_parity_q16")),
            domain.get("regression_count", 0),
        ])
        for domain in domains
    )
    return table(headers, body)


def render_query_table(report: dict[str, Any]) -> str:
    rows = report.get("query_level", [])
    if not isinstance(rows, list):
        rows = []
    headers = ["Query", "Recall", "MRR", "nDCG", "p95 latency", "Exact parity", "Safe"]
    body = "\n".join(
        row([
            item.get("name", item.get("query_id", "unknown")),
            q16_percent(item.get("recall_q16")),
            q16_percent(item.get("mrr_q16")),
            q16_percent(item.get("ndcg_q16")),
            millis(item.get("latency_nanos")),
            "yes" if item.get("exact_parity") else "no",
            "yes" if item.get("production_safe") else "no",
        ])
        for item in rows
        if isinstance(item, dict)
    )
    return table(headers, body)


def table(headers: list[str], body: str) -> str:
    header = "".join(f"<th scope=\"col\">{text(label)}</th>" for label in headers)
    return f"<table><thead><tr>{header}</tr></thead><tbody>{body}</tbody></table>"


def render_dashboard(report: dict[str, Any], beta: dict[str, Any]) -> str:
    failures = validate_beta_report(beta)
    if failures:
        raise ValueError("; ".join(failures))

    guarded = guarded_ann_metrics(report)
    domains = beta.get("domains", [])
    status = beta.get("status", "unknown")
    safe = bool(beta.get("production_safe"))
    tone = "good" if status == "passed" and safe else "bad"
    summary = "\n".join([
        metric("Status", status, tone=tone),
        metric("Production safe", "yes" if safe else "no", tone=tone),
        metric("Domains", beta.get("domain_count", len(domains))),
        metric("Repeat runs", beta.get("repeat_runs_per_domain", "n/a")),
        metric("Top K", beta.get("top_k", "n/a")),
    ])
    ann_summary = "\n".join([
        metric("Recall", q16_percent(guarded.get("mean_recall_q16"))),
        metric("MRR", q16_percent(guarded.get("mean_mrr_q16"))),
        metric("nDCG", q16_percent(guarded.get("mean_ndcg_q16"))),
        metric("p95 latency", millis(guarded.get("p95_latency_nanos"))),
        metric("Exact parity", q16_percent(guarded.get("exact_parity_q16"))),
    ])
    boundary = beta.get("boundary", {})
    proves = boundary.get("proves", "local retrieval evidence")
    does_not = boundary.get("does_not_prove", "production customer relevance")

    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>CortexDB Retrieval Quality Dashboard</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #172026; }}
    h1, h2 {{ margin-bottom: .4rem; }}
    .summary {{ display: grid; gap: .75rem; grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr)); padding: 0; }}
    .summary li {{ list-style: none; border: 1px solid #d3d9de; border-radius: 8px; padding: .75rem; }}
    .summary span {{ display: block; color: #5a6570; font-size: .85rem; }}
    .summary strong {{ font-size: 1.1rem; }}
    .good strong {{ color: #11693a; }}
    .bad strong {{ color: #9a1c1c; }}
    table {{ border-collapse: collapse; width: 100%; margin: 1rem 0 2rem; }}
    th, td {{ border: 1px solid #d3d9de; padding: .55rem .65rem; text-align: left; }}
    th {{ background: #f4f7f9; }}
    .note {{ color: #5a6570; max-width: 72rem; }}
  </style>
</head>
<body>
  <h1>CortexDB Retrieval Quality Dashboard</h1>
  <p class="note">Generated from local evidence reports. Proves: {text(proves)}. Does not prove: {text(does_not)}.</p>
  <h2>Gate Summary</h2>
  <ul class="summary">{summary}</ul>
  <h2>Guarded ANN History</h2>
  <ul class="summary">{ann_summary}</ul>
  <h2>Domain Quality Table</h2>
  {render_domain_table(domains)}
  <h2>Investment Query-Level Table</h2>
  {render_query_table(report)}
</body>
</html>
"""


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--beta-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = load_json(args.report)
        beta = load_json(args.beta_report)
        html_body = render_dashboard(report, beta)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"retrieval quality dashboard failed: {error}", file=sys.stderr)
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(html_body, encoding="utf-8")
    print(f"retrieval quality dashboard written: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
