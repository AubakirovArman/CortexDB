#!/usr/bin/env python3
"""Inventory scale benchmark reports and surface A19 coverage gaps."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


CORE_PHASES = ("put_batches", "checkpoint", "get_latest", "restart_open")
DIRECT_CORE_PHASES = ("direct_checkpoint", "open_prepared", "restart_open")
HEAVY_PHASES = ("keyword_search", "context_pack", "verify_fact")
PERCENTILES = ("p50_ms", "p95_ms")
OPTIMIZATION_HISTORY_EPICS = ("A05", "A06", "A08", "A09")
DEFAULT_OPTIMIZATION_HISTORY = Path("fixtures/scale_bench/optimization_history.json")


def read_report(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def report_paths(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.glob("**/*.json")
        if path.name not in {"inventory.json", "trends.json"} and "history" not in path.parts
    )


def phase_has_percentiles(matrix: dict[str, Any], phase: str) -> bool:
    value = matrix.get(phase)
    return isinstance(value, dict) and all(isinstance(value.get(key), (int, float)) for key in PERCENTILES)


def summarize_report(root: Path, path: Path, report: dict[str, Any]) -> dict[str, Any]:
    matrix = report.get("matrix", {})
    if not isinstance(matrix, dict):
        matrix = {}
    samples = report.get("samples", {})
    if not isinstance(samples, dict):
        samples = {}
    phases = sorted(matrix)
    heavy_ready = {
        phase: phase_has_percentiles(matrix, phase)
        for phase in HEAVY_PHASES
    }
    return {
        "path": str(path.relative_to(root.parent)),
        "ok": report.get("ok") is True,
        "cells": report.get("cells"),
        "payload_profile": report.get("payload_profile"),
        "payload_residency": report.get("payload_residency"),
        "fixture_mode": report.get("fixture_mode", "standard"),
        "samples": samples,
        "phases": phases,
        "has_core_lifecycle": all(phase in matrix for phase in CORE_PHASES)
        or all(phase in matrix for phase in DIRECT_CORE_PHASES),
        "heavy_phase_percentiles": heavy_ready,
    }


def coverage_by_size(reports: list[dict[str, Any]]) -> dict[str, Any]:
    coverage: dict[str, Any] = {}
    for report in reports:
        cells = report.get("cells")
        if not isinstance(cells, int):
            continue
        key = str(cells)
        entry = coverage.setdefault(
            key,
            {"reports": 0, "core_lifecycle": False, "heavy_percentiles": {phase: False for phase in HEAVY_PHASES}},
        )
        entry["reports"] += 1
        entry["core_lifecycle"] = bool(entry["core_lifecycle"] or report["has_core_lifecycle"])
        for phase, ready in report["heavy_phase_percentiles"].items():
            entry["heavy_percentiles"][phase] = bool(entry["heavy_percentiles"][phase] or ready)
    return coverage


def read_optional_optimization_history(path: Path) -> tuple[dict[str, Any] | None, list[str]]:
    if not path.exists():
        return None, []
    try:
        history = read_report(path)
    except Exception as error:  # noqa: BLE001 - inventory should surface unreadable labels.
        return None, [f"{path}: {error}"]
    return history, []


def optimization_history_missing_item(history: dict[str, Any] | None) -> str | None:
    if not isinstance(history, dict):
        return "optimization history: missing before/after A05/A06/A08/A09 curve labels"
    entries = history.get("entries")
    if not isinstance(entries, list):
        return "optimization history: missing before/after A05/A06/A08/A09 curve labels"
    by_epic = {
        entry.get("epic"): entry
        for entry in entries
        if isinstance(entry, dict) and isinstance(entry.get("epic"), str)
    }
    for epic in OPTIMIZATION_HISTORY_EPICS:
        entry = by_epic.get(epic)
        if not isinstance(entry, dict):
            return "optimization history: missing before/after A05/A06/A08/A09 curve labels"
        before = entry.get("before")
        after = entry.get("after")
        if not isinstance(before, dict) or not isinstance(after, dict):
            return "optimization history: missing before/after A05/A06/A08/A09 curve labels"
        if not before.get("label") or not after.get("label"):
            return "optimization history: missing before/after A05/A06/A08/A09 curve labels"
    return None


def trend_missing_items(root: Path) -> list[str]:
    trend_path = root / "trends.json"
    if not trend_path.exists():
        return ["trend curves: missing multi-point scale trend report"]
    try:
        trend = read_report(trend_path)
    except Exception as error:  # noqa: BLE001 - inventory should surface unreadable reports.
        return [f"trend curves: unreadable trend report: {error}"]
    curve_count = trend.get("curve_count")
    if isinstance(curve_count, int) and curve_count > 0:
        return []
    return ["trend curves: trend report has no multi-point curves"]


def missing_items(root: Path, coverage: dict[str, Any], optimization_history: dict[str, Any] | None) -> list[str]:
    missing: list[str] = []
    for cells in ("100000", "1000000"):
        entry = coverage.get(cells)
        if not entry:
            missing.append(f"{cells}: missing scale report")
            continue
        if not entry.get("core_lifecycle"):
            missing.append(f"{cells}: missing core lifecycle phases")
        heavy = entry.get("heavy_percentiles", {})
        for phase in HEAVY_PHASES:
            if not heavy.get(phase):
                missing.append(f"{cells}: missing {phase} p50/p95")
    if "10000000" not in coverage:
        missing.append("10000000: missing post-lazy RSS/latency report")
    missing.extend(trend_missing_items(root))
    history_missing = optimization_history_missing_item(optimization_history)
    if history_missing:
        missing.append(history_missing)
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/scale-bench")
    parser.add_argument("--report", default="target/scale-bench/inventory.json")
    parser.add_argument("--optimization-history", default=str(DEFAULT_OPTIMIZATION_HISTORY))
    args = parser.parse_args()

    root = Path(args.root)
    summaries = []
    errors = []
    for path in report_paths(root):
        try:
            summaries.append(summarize_report(root, path, read_report(path)))
        except Exception as error:  # noqa: BLE001 - inventory should report all unreadable files.
            errors.append(f"{path}: {error}")

    optimization_history, history_errors = read_optional_optimization_history(Path(args.optimization_history))
    errors.extend(history_errors)
    coverage = coverage_by_size(summaries)
    missing = missing_items(root, coverage, optimization_history)
    status = "blocked" if errors else ("complete" if not missing else "partial")
    output = {
        "schema_version": "cortexdb.scale_benchmark_inventory.v1",
        "status": status,
        "reports_found": len(summaries),
        "reports": summaries,
        "coverage_by_cells": coverage,
        "optimization_history": optimization_history,
        "missing_acceptance_items": missing,
        "errors": errors,
    }
    report = Path(args.report)
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"scale benchmark inventory: status={status} reports={len(summaries)} missing={len(missing)}")
    print(f"report: {report}")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
