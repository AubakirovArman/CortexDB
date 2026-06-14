#!/usr/bin/env python3
"""Validate local load/performance reports and compare them with release history."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


LOAD_FLOWS = ("write", "read", "search", "context", "verify")
ENGINE_FLOWS = ("put_single", "get_latest", "keyword_search", "context_pack", "verify_fact")
PERCENTILES = ("p50_ms", "p95_ms", "p99_ms")


def read_json(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise RuntimeError(f"missing report: {path}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"invalid JSON report {path}: {error}") from error


def require_latency(
    errors: list[str], report_name: str, flows: dict[str, Any], required: tuple[str, ...]
) -> None:
    for flow in required:
        summary = flows.get(flow)
        if not isinstance(summary, dict):
            errors.append(f"{report_name}: missing latency flow {flow}")
            continue
        for metric in PERCENTILES:
            if not isinstance(summary.get(metric), (int, float)):
                errors.append(f"{report_name}: {flow} missing numeric {metric}")


def check_thresholds(
    errors: list[str],
    report_name: str,
    summaries: dict[str, Any],
    thresholds: dict[str, Any],
) -> None:
    for flow, flow_thresholds in thresholds.items():
        summary = summaries.get(flow)
        if not isinstance(summary, dict) or not isinstance(flow_thresholds, dict):
            continue
        for metric in PERCENTILES:
            observed = float(summary.get(metric, 0.0))
            allowed = float(flow_thresholds.get(metric, float("inf")))
            if observed > allowed:
                errors.append(
                    f"{report_name}: {flow} {metric} exceeded threshold {observed:.3f}>{allowed:.3f}"
                )


def engine_phase_latencies(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    merged: dict[str, dict[str, Any]] = {}
    for profile in report.get("profiles", []):
        profile_name = str(profile.get("name", "unknown"))
        for phase in profile.get("phases", []):
            name = phase.get("name")
            latency = phase.get("latency")
            if isinstance(name, str) and isinstance(latency, dict):
                merged[f"{profile_name}.{name}"] = latency
    return merged


def check_engine_report(errors: list[str], report: dict[str, Any]) -> None:
    if report.get("ok") is not True:
        errors.append("single-node report is not ok")
    if report.get("workload_class") != "local_single_node_lifecycle":
        errors.append("single-node report missing local_single_node_lifecycle workload_class")
    thresholds = report.get("slo_thresholds", {})
    if not isinstance(thresholds, dict):
        errors.append("single-node report missing slo_thresholds")
    else:
        for field in ("min_ingest_cells_per_sec", "max_rss_bytes"):
            if not isinstance(thresholds.get(field), (int, float)):
                errors.append(f"single-node report missing numeric slo_thresholds.{field}")
    for profile in report.get("profiles", []):
        profile_name = str(profile.get("name", "unknown"))
        phases = {
            phase.get("name"): phase.get("latency")
            for phase in profile.get("phases", [])
            if isinstance(phase, dict)
        }
        require_latency(errors, f"single-node:{profile_name}", phases, ENGINE_FLOWS)
        thresholds = profile.get("latency_thresholds", {})
        if isinstance(thresholds, dict):
            check_thresholds(errors, f"single-node:{profile_name}", phases, thresholds)
        check_engine_profile_slo(errors, profile_name, profile)


def check_engine_profile_slo(errors: list[str], profile_name: str, profile: dict[str, Any]) -> None:
    slo = profile.get("slo")
    if not isinstance(slo, dict):
        errors.append(f"single-node:{profile_name} missing profile slo block")
    elif slo.get("passed") is not True:
        errors.append(f"single-node:{profile_name} profile slo did not pass")
    ingest = profile.get("ingest")
    if not isinstance(ingest, dict):
        errors.append(f"single-node:{profile_name} missing ingest throughput block")
    else:
        throughput = ingest.get("throughput_per_sec")
        minimum = ingest.get("min_throughput_per_sec")
        if not isinstance(throughput, (int, float)):
            errors.append(f"single-node:{profile_name} missing numeric ingest.throughput_per_sec")
        if not isinstance(minimum, (int, float)):
            errors.append(f"single-node:{profile_name} missing numeric ingest.min_throughput_per_sec")
        if isinstance(throughput, (int, float)) and isinstance(minimum, (int, float)):
            if float(throughput) < float(minimum):
                errors.append(
                    f"single-node:{profile_name} ingest throughput below threshold "
                    f"{float(throughput):.3f}<{float(minimum):.3f}"
                )
    resources = profile.get("resource_usage")
    if not isinstance(resources, dict):
        errors.append(f"single-node:{profile_name} missing resource_usage block")
    else:
        for field in ("rss_bytes", "peak_rss_bytes"):
            if not isinstance(resources.get(field), (int, float)):
                errors.append(f"single-node:{profile_name} missing numeric resource_usage.{field}")
        max_rss = profile.get("slo_thresholds", {}).get("max_rss_bytes", None)
        peak = resources.get("peak_rss_bytes", None)
        if isinstance(max_rss, (int, float)) and isinstance(peak, (int, float)):
            if float(peak) > float(max_rss):
                errors.append(
                    f"single-node:{profile_name} peak RSS exceeded threshold "
                    f"{float(peak):.0f}>{float(max_rss):.0f}"
                )


def check_load_report(errors: list[str], report: dict[str, Any]) -> None:
    if report.get("ok") is not True:
        errors.append("load smoke report is not ok")
    if report.get("workload_class") != "local_http_smoke":
        errors.append("load smoke report missing local_http_smoke workload_class")
    latencies = report.get("latencies", {})
    if not isinstance(latencies, dict):
        errors.append("load smoke report missing latencies")
        return
    require_latency(errors, "load-smoke", latencies, LOAD_FLOWS)
    thresholds = report.get("latency_thresholds", {})
    if isinstance(thresholds, dict):
        check_thresholds(errors, "load-smoke", latencies, thresholds)
    actor = report.get("actor", {})
    if not isinstance(actor, dict):
        errors.append("load smoke report missing actor metrics")
        return
    for field in ("queue_depth", "queue_capacity", "queue_saturation", "database_busy_count"):
        if field not in actor:
            errors.append(f"load smoke actor metrics missing {field}")
    if int(actor.get("database_busy_count", 0)) != 0:
        errors.append("load smoke observed database_busy/request rejection")


def collect_history(history_root: Path) -> list[dict[str, Any]]:
    runs: list[dict[str, Any]] = []
    if not history_root.exists():
        return runs
    for release_dir in sorted(path for path in history_root.iterdir() if path.is_dir()):
        load_path = release_dir / "load_smoke_report.json"
        engine_path = release_dir / "single_node_performance_report.json"
        if load_path.exists() and engine_path.exists():
            runs.append(
                {
                    "release": release_dir.name,
                    "load": read_json(load_path),
                    "single_node": read_json(engine_path),
                }
            )
    return runs


def ratio(current: float, previous: float) -> float:
    if previous <= 0:
        return 0.0
    return round(current / previous, 6)


def comparison_detail(current: float, previous: float) -> dict[str, float]:
    return {
        "current_ms": round(current, 6),
        "previous_ms": round(previous, 6),
        "delta_ms": round(current - previous, 6),
        "ratio": ratio(current, previous),
    }


def compare_load(
    current: dict[str, Any], previous: dict[str, Any], *, detailed: bool = False
) -> dict[str, Any]:
    comparisons: dict[str, Any] = {}
    for flow in LOAD_FLOWS:
        current_flow = current.get("latencies", {}).get(flow, {})
        previous_flow = previous.get("latencies", {}).get(flow, {})
        comparisons[flow] = {
            metric: comparison_detail(
                float(current_flow.get(metric, 0.0)), float(previous_flow.get(metric, 0.0))
            )
            if detailed
            else ratio(float(current_flow.get(metric, 0.0)), float(previous_flow.get(metric, 0.0)))
            for metric in PERCENTILES
        }
    return comparisons


def compare_engine(
    current: dict[str, Any], previous: dict[str, Any], *, detailed: bool = False
) -> dict[str, Any]:
    current_flows = engine_phase_latencies(current)
    previous_flows = engine_phase_latencies(previous)
    comparisons: dict[str, Any] = {}
    for flow, current_latency in current_flows.items():
        previous_latency = previous_flows.get(flow, {})
        comparisons[flow] = {
            metric: comparison_detail(
                float(current_latency.get(metric, 0.0)), float(previous_latency.get(metric, 0.0))
            )
            if detailed
            else ratio(float(current_latency.get(metric, 0.0)), float(previous_latency.get(metric, 0.0)))
            for metric in PERCENTILES
        }
    return comparisons


def current_engine_slo_summary(current: dict[str, Any]) -> dict[str, Any]:
    profiles: dict[str, Any] = {}
    for profile in current.get("profiles", []):
        name = str(profile.get("name", "unknown"))
        profiles[name] = {
            "slo_passed": profile.get("slo", {}).get("passed"),
            "ingest_throughput_per_sec": profile.get("ingest", {}).get("throughput_per_sec"),
            "rss_bytes": profile.get("resource_usage", {}).get("rss_bytes"),
            "peak_rss_bytes": profile.get("resource_usage", {}).get("peak_rss_bytes"),
        }
    return profiles


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--load-report", default="target/load-smoke/report.json")
    parser.add_argument("--single-node-report", default="target/single-node-performance/report.json")
    parser.add_argument("--history-root", default="fixtures/performance/history")
    parser.add_argument("--report", default="target/performance-trends/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    load_report = read_json(Path(args.load_report))
    single_node_report = read_json(Path(args.single_node_report))
    history = collect_history(Path(args.history_root))
    errors: list[str] = []
    if not history:
        errors.append(f"no release performance history found under {args.history_root}")
    check_load_report(errors, load_report)
    check_engine_report(errors, single_node_report)

    latest = history[-1] if history else None
    report = {
        "schema_version": "cortexdb.performance_trends.v1",
        "status": "passed" if not errors else "failed",
        "history_runs": [run["release"] for run in history],
        "current": {
            "load_report": args.load_report,
            "single_node_report": args.single_node_report,
        },
        "comparisons_to_latest_history": {
            "release": latest["release"] if latest else None,
            "load_p50_p95_p99_ratio": compare_load(load_report, latest["load"]) if latest else {},
            "load_p50_p95_p99_details": compare_load(load_report, latest["load"], detailed=True)
            if latest
            else {},
            "single_node_p50_p95_p99_ratio": compare_engine(
                single_node_report, latest["single_node"]
            )
            if latest
            else {},
            "single_node_p50_p95_p99_details": compare_engine(
                single_node_report, latest["single_node"], detailed=True
            )
            if latest
            else {},
        },
        "single_node_slo_summary": current_engine_slo_summary(single_node_report),
        "errors": errors,
    }
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if errors:
        print(f"performance trend check failed: {report_path}", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print(f"performance trend check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
