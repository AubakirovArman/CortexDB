"""Shared latency helpers for local CortexDB performance gates."""

from __future__ import annotations


def percentile(values: list[float], percent: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, int((len(ordered) - 1) * percent))
    return ordered[index]


def latency_summary(values: list[float]) -> dict[str, float | int]:
    return {
        "count": len(values),
        "p50_ms": round(percentile(values, 0.50), 3),
        "p95_ms": round(percentile(values, 0.95), 3),
        "p99_ms": round(percentile(values, 0.99), 3),
        "max_ms": round(max(values) if values else 0.0, 3),
    }


def load_smoke_latency_thresholds() -> dict[str, dict[str, float]]:
    return {
        "write": {"p95_ms": 500.0, "p99_ms": 1000.0},
        "read": {"p95_ms": 250.0, "p99_ms": 500.0},
        "search": {"p95_ms": 1000.0, "p99_ms": 2000.0},
        "context": {"p95_ms": 1500.0, "p99_ms": 3000.0},
        "verify": {"p95_ms": 1500.0, "p99_ms": 3000.0},
    }


def check_latency_thresholds(
    summaries: dict[str, dict[str, float | int]],
    thresholds: dict[str, dict[str, float]],
) -> list[str]:
    errors: list[str] = []
    for flow, flow_thresholds in thresholds.items():
        summary = summaries.get(flow)
        if summary is None:
            errors.append(f"missing latency summary for {flow}")
            continue
        for metric, threshold in flow_thresholds.items():
            observed = float(summary.get(metric, 0.0))
            if observed > threshold:
                errors.append(
                    f"{flow} {metric} exceeded threshold: {observed:.3f} > {threshold:.3f}"
                )
    return errors
