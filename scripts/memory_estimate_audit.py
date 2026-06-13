#!/usr/bin/env python3
"""Audit estimated memory against observed RSS in existing benchmark reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


RESOURCE_PHASES = ("after_checkpoint", "after_open_prepared", "after_put")


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def ratio(numerator: float, denominator: float) -> float | None:
    if denominator <= 0:
        return None
    return round(numerator / denominator, 6)


def row(
    *,
    root: Path,
    path: Path,
    source: str,
    phase: str,
    report: dict[str, Any],
    values: dict[str, Any],
) -> dict[str, Any] | None:
    estimated = values.get("estimated_total_memory_bytes")
    rss = values.get("rss_bytes")
    peak = values.get("peak_rss_bytes")
    if not isinstance(estimated, (int, float)) or not isinstance(rss, (int, float)):
        return None
    peak_value = float(peak) if isinstance(peak, (int, float)) else None
    return {
        "source": source,
        "phase": phase,
        "path": str(path.relative_to(root.parent)),
        "cells": report.get("cells"),
        "payload_residency": report.get("payload_residency"),
        "payload_profile": report.get("payload_profile"),
        "fixture_mode": report.get("fixture_mode", report.get("mode")),
        "estimated_total_memory_bytes": int(estimated),
        "rss_bytes": int(rss),
        "peak_rss_bytes": int(peak_value) if peak_value is not None else None,
        "rss_to_estimated_total_ratio": ratio(float(rss), float(estimated)),
        "peak_rss_to_estimated_total_ratio": ratio(peak_value, float(estimated))
        if peak_value is not None
        else None,
    }


def memory_profile_rows(root: Path) -> tuple[list[dict[str, Any]], list[str]]:
    rows: list[dict[str, Any]] = []
    errors: list[str] = []
    if not root.exists():
        return rows, [f"missing memory profile root: {root}"]
    for path in sorted(root.glob("**/*.json")):
        try:
            report = read_json(path)
        except Exception as error:  # noqa: BLE001 - audit should report all bad files.
            errors.append(f"{path}: {error}")
            continue
        values = report.get("estimate_vs_rss")
        if not isinstance(values, dict):
            continue
        item = row(root=root, path=path, source="memory_profile", phase="final", report=report, values=values)
        if item is not None:
            rows.append(item)
    return rows, errors


def scale_benchmark_rows(root: Path) -> tuple[list[dict[str, Any]], list[str]]:
    rows: list[dict[str, Any]] = []
    errors: list[str] = []
    if not root.exists():
        return rows, [f"missing scale benchmark root: {root}"]
    for path in sorted(root.glob("**/*.json")):
        if path.name in {"inventory.json", "trends.json"} or "history" in path.parts:
            continue
        try:
            report = read_json(path)
        except Exception as error:  # noqa: BLE001 - audit should report all bad files.
            errors.append(f"{path}: {error}")
            continue
        matrix = report.get("matrix", {})
        if not isinstance(matrix, dict):
            continue
        for phase in RESOURCE_PHASES:
            values = matrix.get(phase)
            if not isinstance(values, dict):
                continue
            item = row(root=root, path=path, source="scale_benchmark", phase=phase, report=report, values=values)
            if item is not None:
                rows.append(item)
    return rows, errors


def summarize(rows: list[dict[str, Any]], threshold: float) -> dict[str, Any]:
    max_rss = max(
        (row["rss_to_estimated_total_ratio"] or 0.0 for row in rows),
        default=0.0,
    )
    max_peak = max(
        (row["peak_rss_to_estimated_total_ratio"] or 0.0 for row in rows),
        default=0.0,
    )
    violations = [
        row
        for row in rows
        if (row["rss_to_estimated_total_ratio"] or 0.0) > threshold
        or (row["peak_rss_to_estimated_total_ratio"] or 0.0) > threshold
    ]
    return {
        "rows": len(rows),
        "max_rss_to_estimated_total_ratio": round(max_rss, 6),
        "max_peak_rss_to_estimated_total_ratio": round(max_peak, 6),
        "threshold": threshold,
        "violations": len(violations),
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Memory Estimate Audit",
        "",
        f"Status: `{report['status']}`",
        f"Rows: `{report['summary']['rows']}`",
        f"Max RSS/estimated: `{report['summary']['max_rss_to_estimated_total_ratio']}`",
        f"Max peak RSS/estimated: `{report['summary']['max_peak_rss_to_estimated_total_ratio']}`",
        "",
        "| source | cells | phase | residency | rss/estimated | peak/estimated | path |",
        "| --- | ---: | --- | --- | ---: | ---: | --- |",
    ]
    for item in report["rows"]:
        lines.append(
            "| {source} | {cells} | {phase} | {residency} | {rss} | {peak} | `{path}` |".format(
                source=item["source"],
                cells=item.get("cells"),
                phase=item["phase"],
                residency=item.get("payload_residency"),
                rss=item.get("rss_to_estimated_total_ratio"),
                peak=item.get("peak_rss_to_estimated_total_ratio"),
                path=item["path"],
            )
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--memory-root", default="target/memory-profile")
    parser.add_argument("--scale-root", default="target/scale-bench")
    parser.add_argument("--report", default="target/memory-profile/estimate-audit.json")
    parser.add_argument("--markdown", default="target/memory-profile/estimate-audit.md")
    parser.add_argument("--max-rss-to-estimated-total-ratio", type=float, default=128.0)
    args = parser.parse_args()

    memory_rows, memory_errors = memory_profile_rows(Path(args.memory_root))
    scale_rows, scale_errors = scale_benchmark_rows(Path(args.scale_root))
    rows = sorted(
        memory_rows + scale_rows,
        key=lambda item: (str(item.get("source")), int(item.get("cells") or 0), item["path"], item["phase"]),
    )
    errors = memory_errors + scale_errors
    summary = summarize(rows, args.max_rss_to_estimated_total_ratio)
    if not rows:
        errors.append("no comparable memory estimate rows found")
    status = "passed" if not errors and summary["violations"] == 0 else "failed"
    report = {
        "schema_version": "cortexdb.memory_estimate_audit.v1",
        "status": status,
        "summary": summary,
        "rows": rows,
        "errors": errors,
    }
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(Path(args.markdown), report)
    print(
        "memory estimate audit: "
        f"status={status} rows={summary['rows']} "
        f"max_rss_ratio={summary['max_rss_to_estimated_total_ratio']}"
    )
    print(f"report: {report_path}")
    print(f"markdown: {args.markdown}")
    return 0 if status == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
