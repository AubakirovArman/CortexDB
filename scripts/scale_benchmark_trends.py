#!/usr/bin/env python3
"""Build scale benchmark trend curves from existing report JSON files."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


PHASE_METRICS = {
    "checkpoint": ("elapsed_ms",),
    "direct_checkpoint": ("elapsed_ms",),
    "get_latest": ("p50_ms", "p95_ms"),
    "keyword_search": ("p50_ms", "p95_ms"),
    "context_pack": ("p50_ms", "p95_ms"),
    "verify_fact": ("p50_ms", "p95_ms"),
    "open_empty": ("elapsed_ms",),
    "open_prepared": ("elapsed_ms",),
    "restart_open": ("elapsed_ms",),
}
RESOURCE_PHASES = ("after_checkpoint", "after_open_prepared", "after_put")
RESOURCE_METRICS = ("rss_bytes", "peak_rss_bytes", "estimated_total_memory_bytes")


def read_report(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def report_paths(root: Path) -> list[Path]:
    ignored = {"inventory.json", "trends.json"}
    return sorted(
        path
        for path in root.glob("**/*.json")
        if path.name not in ignored and "history" not in path.parts
    )


def git_revision() -> str | None:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:  # noqa: BLE001 - trend reports should still work outside git.
        return None


def samples_signature(samples: dict[str, Any]) -> str:
    parts = []
    for key in ("read", "search", "context", "verify"):
        value = samples.get(key, 0)
        if isinstance(value, int) and value:
            parts.append(f"{key}{value}")
    return ",".join(parts) if parts else "no-samples"


def report_profile(report: dict[str, Any], path: Path) -> str:
    samples = report.get("samples", {})
    if not isinstance(samples, dict):
        samples = {}
    profile_parts = [
        str(report.get("fixture_mode", "standard")),
        str(report.get("payload_profile", "unknown-payload")),
        str(report.get("payload_residency", "default-residency")),
        samples_signature(samples),
    ]
    return "|".join(profile_parts)


def add_point(
    series: dict[str, Any],
    profile: str,
    phase: str,
    metric: str,
    cells: int,
    value: float,
    path: Path,
    root: Path,
) -> None:
    key = f"{profile}|{phase}|{metric}"
    entry = series.setdefault(
        key,
        {
            "profile": profile,
            "phase": phase,
            "metric": metric,
            "points": [],
        },
    )
    entry["points"].append(
        {
            "cells": cells,
            "value": value,
            "report": str(path.relative_to(root.parent)),
        }
    )


def collect_series(root: Path) -> tuple[list[dict[str, Any]], list[str]]:
    series: dict[str, Any] = {}
    errors: list[str] = []
    for path in report_paths(root):
        try:
            report = read_report(path)
        except Exception as error:  # noqa: BLE001 - report all unreadable files.
            errors.append(f"{path}: {error}")
            continue
        cells = report.get("cells")
        if not isinstance(cells, int):
            continue
        matrix = report.get("matrix", {})
        if not isinstance(matrix, dict):
            continue
        profile = report_profile(report, path)
        for phase, metrics in PHASE_METRICS.items():
            phase_value = matrix.get(phase)
            if not isinstance(phase_value, dict):
                continue
            for metric in metrics:
                value = phase_value.get(metric)
                if isinstance(value, (int, float)):
                    add_point(series, profile, phase, metric, cells, float(value), path, root)
        for phase in RESOURCE_PHASES:
            phase_value = matrix.get(phase)
            if not isinstance(phase_value, dict):
                continue
            for metric in RESOURCE_METRICS:
                value = phase_value.get(metric)
                if isinstance(value, (int, float)):
                    add_point(series, profile, phase, metric, cells, float(value), path, root)

    curves = []
    for entry in series.values():
        points = sorted(entry["points"], key=lambda point: (point["cells"], point["report"]))
        cells_seen = {point["cells"] for point in points}
        if len(cells_seen) < 2:
            continue
        first = points[0]["value"]
        last = points[-1]["value"]
        ratio = None if first <= 0 else round(last / first, 6)
        curves.append({**entry, "points": points, "first_to_last_ratio": ratio})
    curves.sort(key=lambda item: (item["phase"], item["metric"], item["profile"]))
    return curves, errors


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Scale Benchmark Trends",
        "",
        f"Status: `{report['status']}`",
        f"Git revision: `{report.get('git_revision') or 'unknown'}`",
        f"Curves: `{len(report['curves'])}`",
        "",
        "## Missing Items",
        "",
    ]
    for item in report["missing_acceptance_items"]:
        lines.append(f"- {item}")
    lines.extend(["", "## Curves", ""])
    for curve in report["curves"]:
        lines.append(
            f"### `{curve['phase']}.{curve['metric']}` "
            f"({curve['profile']}, ratio={curve['first_to_last_ratio']})"
        )
        lines.extend(["", "| cells | value | report |", "| ---: | ---: | --- |"])
        for point in curve["points"]:
            lines.append(f"| {point['cells']} | {point['value']} | `{point['report']}` |")
        lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/scale-bench")
    parser.add_argument("--report", default="target/scale-bench/trends.json")
    parser.add_argument("--markdown", default="target/scale-bench/trends.md")
    args = parser.parse_args()

    root = Path(args.root)
    curves, errors = collect_series(root)
    has_10m = any(point["cells"] >= 10_000_000 for curve in curves for point in curve["points"])
    missing = []
    if not curves:
        missing.append("no multi-point scale curves found")
    if not has_10m:
        missing.append("10000000: missing post-lazy RSS/latency curve point")
    missing.append("optimization history: missing before/after A05/A06/A08/A09 curve labels")
    status = "blocked" if errors else ("partial" if missing else "complete")
    report = {
        "schema_version": "cortexdb.scale_benchmark_trends.v1",
        "status": status,
        "git_revision": git_revision(),
        "curve_count": len(curves),
        "curves": curves,
        "missing_acceptance_items": missing,
        "errors": errors,
    }
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(Path(args.markdown), report)
    print(f"scale benchmark trends: status={status} curves={len(curves)} missing={len(missing)}")
    print(f"report: {report_path}")
    print(f"markdown: {args.markdown}")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
