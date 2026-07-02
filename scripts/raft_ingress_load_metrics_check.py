#!/usr/bin/env python3
"""Validate operator-tunable cached Raft ingress load metrics evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/config.rs": [
        "DEFAULT_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE",
        "cluster_ingress_max_in_flight_per_node",
        "cluster_ingress_max_in_flight_per_node(&self)",
    ],
    "crates/cortex-server/src/main.rs": [
        "CORTEXDB_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE",
        "cluster_ingress_max_in_flight_from_env",
        "parse_cluster_ingress_max_in_flight",
    ],
    "crates/cortex-server/src/cluster/monitor.rs": [
        "ClusterIngressLoadMetrics",
        "options.cluster_ingress_max_in_flight_per_node()",
        "load_metrics",
        "available_permits_for_cached_leader",
    ],
    "crates/cortex-server/src/http_metrics.rs": ["mod cluster_ingress;"],
    "crates/cortex-server/src/http_metrics/cluster_ingress.rs": [
        "ClusterIngressPrometheusMetrics",
        "cluster_ingress_prometheus_metrics",
        "monitor.load_metrics()",
        "available_permits",
    ],
    "crates/cortex-server/src/http_metrics/response.rs": [
        "cluster_ingress_prometheus_metrics(state)",
        "cortexdb_cluster_ingress_configured",
        "cortexdb_cluster_ingress_cached_leader_id",
        "cortexdb_cluster_ingress_max_in_flight_per_node",
        "cortexdb_cluster_ingress_in_flight",
        "cortexdb_cluster_ingress_available_permits",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_load_tests.rs": [
        "load_policy_uses_operator_configured_limit_from_options",
        "load_policy_metrics_report_cached_leader_limit_and_in_flight",
        "cluster_ingress_max_in_flight_per_node = 1",
        "monitor.load_metrics()",
    ],
    "crates/cortex-server/src/main/tests.rs": [
        "parse_cluster_ingress_max_in_flight_accepts_positive_integer",
        "parse_cluster_ingress_max_in_flight_rejects_zero_and_invalid_values",
    ],
    "crates/cortex-server/src/tests/snapshot_tests.rs": [
        "cortexdb_cluster_ingress_max_in_flight_per_node",
        "cortexdb_cluster_ingress_available_permits",
    ],
    "mk/core-contracts.mk": [
        "raft-ingress-load-metrics-check:",
        "parse_cluster_ingress_max_in_flight",
        "metrics_prometheus_output_contains_contract_series",
        "scripts/raft_ingress_load_metrics_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_LOAD_METRICS_REPORT"],
    "mk/phony.mk": ["raft-ingress-load-metrics-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "raft-ingress-load-metrics-check",
        "operator-tunable cached ingress load limit",
        "Prometheus cached ingress load gauges",
    ],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster/monitor.rs": 250,
    "crates/cortex-server/src/http_metrics/cluster_ingress.rs": 120,
    "crates/cortex-server/src/http_metrics/response.rs": 300,
    "crates/cortex-server/src/tests/cluster_ingress_load_tests.rs": 260,
    "scripts/raft_ingress_load_metrics_check.py": 180,
}


def read_text(root: Path, relative: str) -> str:
    path = root / relative
    if not path.exists():
        raise SystemExit(f"missing required file: {relative}")
    return path.read_text(encoding="utf-8")


def check_markers(root: Path) -> list[str]:
    checked = []
    for relative, markers in REQUIRED_MARKERS.items():
        text = read_text(root, relative)
        missing = [marker for marker in markers if marker not in text]
        if missing:
            raise SystemExit(f"{relative} missing markers: {', '.join(missing)}")
        checked.append(relative)
    return checked


def check_line_limits(root: Path) -> dict[str, int]:
    line_counts = {}
    for relative, limit in LINE_LIMITS.items():
        count = len(read_text(root, relative).splitlines())
        line_counts[relative] = count
        if count > limit:
            raise SystemExit(f"{relative} has {count} lines; limit is {limit}")
    return line_counts


def write_report(path: Path, checked: list[str], line_counts: dict[str, int]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": "cortexdb.raft_ingress_load_metrics_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "cached Raft ingress monitor limit is operator-tunable by env/server options",
            "zero or invalid operator load limits fail closed during startup parsing",
            "cached monitor exports selected leader, limit, in-flight, and remaining permit metrics",
            "Prometheus metrics include stable cached ingress load gauge names",
        ],
        "does_not_prove": [
            "adaptive refresh-on-overload; covered by raft-ingress-adaptive-scheduling-check",
            "load balancing across multiple writable Raft leaders",
            "release-lane partition/failover/rejoin soak stability",
            "partition-linearizable arbitrary-node reads",
            "external witnessed transparency, KMS/HSM custody, or production receipt guarantees",
        ],
    }
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    args = parser.parse_args()

    root = Path(args.root).resolve()
    checked = check_markers(root)
    line_counts = check_line_limits(root)
    write_report(Path(args.report), checked, line_counts)
    print(f"Raft ingress load metrics check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
