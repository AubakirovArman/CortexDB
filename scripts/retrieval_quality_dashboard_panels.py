#!/usr/bin/env python3
"""Metric panels for the retrieval quality dashboard."""

from __future__ import annotations

import html
from typing import Any

Q16_ONE = 65_535


def text(value: Any) -> str:
    return html.escape(str(value))


def q16_percent(value: Any) -> str:
    if not isinstance(value, int):
        return "n/a"
    return f"{(value * 100) / Q16_ONE:.2f}%"


def q16_width(value: Any) -> int:
    if not isinstance(value, int):
        return 0
    return max(0, min(100, round((value * 100) / Q16_ONE)))


def millis(value: Any) -> str:
    if not isinstance(value, int):
        return "n/a"
    return f"{value / 1_000_000:.3f} ms"


def row(cells: list[Any]) -> str:
    return "<tr>" + "".join(f"<td>{text(cell)}</td>" for cell in cells) + "</tr>"


def table(headers: list[str], body: str) -> str:
    header = "".join(f'<th scope="col">{text(label)}</th>' for label in headers)
    return f"<table><thead><tr>{header}</tr></thead><tbody>{body}</tbody></table>"


def panel(title: str, body: str) -> str:
    return f'<section class="panel"><h3>{text(title)}</h3>{body}</section>'


def render_quality_panel(domains: list[dict[str, Any]], title: str, field: str) -> str:
    rows = []
    for domain in domains:
        name = domain.get("domain", "unknown")
        value = domain.get(field)
        width = q16_width(value)
        rows.append(
            f"<li><span>{text(name)}</span><strong>{q16_percent(value)}</strong>"
            f'<div class="bar" aria-label="{text(name)} {text(title)}">'
            f'<i style="inline-size: {width}%"></i></div></li>'
        )
    return panel(title, f'<ol class="metric-bars">{"".join(rows)}</ol>')


def previous_domain_run(history: dict[str, Any], domain: str, latest_id: Any) -> dict[str, Any]:
    runs = [
        run
        for run in history.get("runs", [])
        if isinstance(run, dict) and run.get("domain") == domain
    ]
    for index, run in enumerate(runs):
        if run.get("run_id") == latest_id and index > 0:
            previous = runs[index - 1]
            return previous if isinstance(previous, dict) else {}
    return {}


def render_latency_panel(domains: list[dict[str, Any]], history: dict[str, Any]) -> str:
    headers = ["Domain", "Latest p95", "Previous p95", "Delta", "Trend"]
    rows = []
    history_domains = {
        item.get("domain"): item
        for item in history.get("domains", [])
        if isinstance(item, dict) and item.get("domain")
    }
    for domain in domains:
        name = str(domain.get("domain", "unknown"))
        history_row = history_domains.get(name, {})
        latest = history_row.get("latest_p95_latency_nanos", domain.get("latest_p95_latency_nanos"))
        previous = previous_domain_run(history, name, history_row.get("latest_run_id"))
        previous_value = previous.get("p95_latency_nanos")
        delta = int(latest) - int(previous_value) if isinstance(latest, int) and isinstance(previous_value, int) else None
        trend = "stable"
        if isinstance(delta, int) and delta > 0:
            trend = "slower"
        elif isinstance(delta, int) and delta < 0:
            trend = "faster"
        rows.append(
            row([
                name,
                millis(latest),
                millis(previous_value),
                "n/a" if delta is None else millis(abs(delta)),
                trend,
            ])
        )
    return panel("Latency Trend Panel", table(headers, "\n".join(rows)))


def render_metric_panels(beta: dict[str, Any], history: dict[str, Any]) -> str:
    domains = beta.get("domains", [])
    if not isinstance(domains, list):
        domains = []
    panels = [
        render_quality_panel(domains, "Recall Panel", "latest_mean_recall_q16"),
        render_quality_panel(domains, "MRR Panel", "latest_mean_mrr_q16"),
        render_quality_panel(domains, "nDCG Panel", "latest_mean_ndcg_q16"),
        render_latency_panel(domains, history),
    ]
    return f'<div class="panel-grid">{"".join(panels)}</div>'
