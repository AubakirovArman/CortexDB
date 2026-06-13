#!/usr/bin/env python3
"""Gate existing benchmark trend artifacts without running heavy benchmarks."""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path
from typing import Any


RATIO_METRICS = ("p95_ms", "p99_ms")


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def ratio_violations(report: dict[str, Any], max_ratio: float) -> list[dict[str, Any]]:
    comparisons = report.get("comparisons_to_latest_history", {})
    if not isinstance(comparisons, dict):
        return [{"kind": "missing_comparisons", "message": "missing comparisons_to_latest_history"}]
    violations: list[dict[str, Any]] = []
    for group, flows in comparisons.items():
        if group == "release" or not isinstance(flows, dict):
            continue
        for flow, metrics in flows.items():
            if not isinstance(metrics, dict):
                continue
            for metric in RATIO_METRICS:
                value = metrics.get(metric)
                if isinstance(value, (int, float)) and float(value) > max_ratio:
                    violations.append(
                        {
                            "kind": "ratio_exceeded",
                            "group": group,
                            "flow": flow,
                            "metric": metric,
                            "ratio": float(value),
                            "max_ratio": max_ratio,
                        }
                    )
    return violations


def validate_performance_trend(path: Path, max_ratio: float) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    report = read_json(path)
    if report.get("status") != "passed":
        errors.append(f"{path}: status is not passed")
    history_runs = report.get("history_runs")
    if not isinstance(history_runs, list) or not history_runs:
        errors.append(f"{path}: missing history runs")
    violations = ratio_violations(report, max_ratio)
    if violations:
        errors.append(f"{path}: {len(violations)} p95/p99 ratio violations")
    return {
        "path": str(path),
        "status": report.get("status"),
        "history_runs": history_runs,
        "latest_history": report.get("comparisons_to_latest_history", {}).get("release"),
        "violations": violations,
    }, errors


def validate_scale_trends(path: Path) -> tuple[dict[str, Any], list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    if not path.exists():
        warnings.append(f"{path}: missing scale trend artifact")
        return {"path": str(path), "status": "missing", "curve_count": 0}, errors, warnings
    report = read_json(path)
    curve_count = report.get("curve_count")
    if not isinstance(curve_count, int) or curve_count <= 0:
        errors.append(f"{path}: no scale curves")
    if report.get("errors"):
        errors.append(f"{path}: contains trend errors")
    if report.get("status") != "complete":
        warnings.append(f"{path}: status={report.get('status')} retained as long-run evidence")
    return {
        "path": str(path),
        "status": report.get("status"),
        "curve_count": curve_count,
        "missing_acceptance_items": report.get("missing_acceptance_items", []),
    }, errors, warnings


def validate_memory_audit(path: Path) -> tuple[dict[str, Any], list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    if not path.exists():
        warnings.append(f"{path}: missing memory audit artifact")
        return {"path": str(path), "status": "missing"}, errors, warnings
    report = read_json(path)
    if report.get("status") != "passed":
        errors.append(f"{path}: status is not passed")
    summary = report.get("summary", {})
    if not isinstance(summary, dict) or int(summary.get("rows", 0)) <= 0:
        errors.append(f"{path}: missing comparable rows")
    return {"path": str(path), "status": report.get("status"), "summary": summary}, errors, warnings


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Continuous Benchmark Gate",
        "",
        f"Status: `{report['status']}`",
        f"Max p95/p99 ratio: `{report['max_ratio']}`",
        "",
        "## Inputs",
        "",
    ]
    for name, value in report["inputs"].items():
        lines.append(f"- {name}: `{value.get('path')}` status=`{value.get('status')}`")
    lines.extend(["", "## Errors", ""])
    lines.extend(f"- {error}" for error in report["errors"]) or lines.append("- none")
    lines.extend(["", "## Warnings", ""])
    lines.extend(f"- {warning}" for warning in report["warnings"]) or lines.append("- none")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_gate(args: argparse.Namespace) -> int:
    errors: list[str] = []
    warnings: list[str] = []
    performance, performance_errors = validate_performance_trend(
        Path(args.performance_trend_report), args.max_p95_ratio
    )
    scale, scale_errors, scale_warnings = validate_scale_trends(Path(args.scale_trend_report))
    memory, memory_errors, memory_warnings = validate_memory_audit(Path(args.memory_audit_report))
    errors.extend(performance_errors + scale_errors + memory_errors)
    warnings.extend(scale_warnings + memory_warnings)
    status = "passed" if not errors else "failed"
    report = {
        "schema_version": "cortexdb.continuous_benchmark_gate.v1",
        "status": status,
        "max_ratio": args.max_p95_ratio,
        "inputs": {
            "performance_trend": performance,
            "scale_trends": scale,
            "memory_audit": memory,
        },
        "errors": errors,
        "warnings": warnings,
    }
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(Path(args.markdown), report)
    print(f"continuous benchmark gate: status={status} errors={len(errors)} warnings={len(warnings)}")
    print(f"report: {report_path}")
    print(f"markdown: {args.markdown}")
    return 0 if status == "passed" else 1


def self_test() -> int:
    report = {
        "status": "passed",
        "history_runs": ["baseline"],
        "comparisons_to_latest_history": {
            "release": "baseline",
            "single_node_p50_p95_p99_ratio": {
                "strict.context_pack": {"p95_ms": 1.25, "p99_ms": 1.0}
            },
        },
    }
    with tempfile.TemporaryDirectory() as temp:
        path = Path(temp) / "trend.json"
        path.write_text(json.dumps(report), encoding="utf-8")
        _, errors = validate_performance_trend(path, 1.2)
    if not errors:
        print("continuous benchmark gate self-test failed: synthetic regression was not detected")
        return 1
    print("continuous benchmark gate self-test passed")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--performance-trend-report", default="target/performance-trends/report.json")
    parser.add_argument("--scale-trend-report", default="target/scale-bench/trends.json")
    parser.add_argument("--memory-audit-report", default="target/memory-profile/estimate-audit.json")
    parser.add_argument("--report", default="target/continuous-benchmark-gate/report.json")
    parser.add_argument("--markdown", default="target/continuous-benchmark-gate/report.md")
    parser.add_argument("--max-p95-ratio", type=float, default=1.2)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        return self_test()
    return run_gate(args)


if __name__ == "__main__":
    raise SystemExit(main())
