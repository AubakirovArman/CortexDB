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


def read_report(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def report_paths(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.glob("**/*.json")
        if path.name != "inventory.json" and "history" not in path.parts
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


def missing_items(coverage: dict[str, Any]) -> list[str]:
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
    missing.append("trend curves: missing multi-point before/after optimization history")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/scale-bench")
    parser.add_argument("--report", default="target/scale-bench/inventory.json")
    args = parser.parse_args()

    root = Path(args.root)
    summaries = []
    errors = []
    for path in report_paths(root):
        try:
            summaries.append(summarize_report(root, path, read_report(path)))
        except Exception as error:  # noqa: BLE001 - inventory should report all unreadable files.
            errors.append(f"{path}: {error}")

    coverage = coverage_by_size(summaries)
    missing = missing_items(coverage)
    status = "blocked" if errors else ("complete" if not missing else "partial")
    output = {
        "schema_version": "cortexdb.scale_benchmark_inventory.v1",
        "status": status,
        "reports_found": len(summaries),
        "reports": summaries,
        "coverage_by_cells": coverage,
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
