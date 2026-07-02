#!/usr/bin/env python3
"""Validate the live Raft ingress production guard evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/cluster.rs": [
        "LIVE_RAFT_INGRESS_UNAVAILABLE",
        "status_response",
        "context_ingress_decision",
        "ContextIngressDecision::Unavailable",
        "ClusterStatusResponse",
        "ClusterConfig::single_node",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs": [
        "cluster_status_uses_configured_multi_node_topology",
        "non_primary_context_route_fails_closed_when_primary_is_unavailable",
        "handle_http_with_options",
        "crate::handler::axum_handler",
        "service_unavailable",
    ],
    "crates/cortex-server/src/sync_handler.rs": [
        "crate::cluster::status_response",
        "crate::cluster::context_ingress_decision",
        "ContextIngressDecision::Unavailable",
    ],
    "crates/cortex-server/src/handler.rs": [
        "crate::cluster::status_response",
        "crate::cluster::context_ingress_decision",
        "live_raft_ingress_unavailable_response",
    ],
    "mk/core-contracts.mk": [
        "raft-ingress-production-guard-check:",
        "cargo test -p cortex-server cluster_ingress_guard_tests --all-features",
        "scripts/raft_ingress_production_guard_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_PRODUCTION_GUARD_REPORT"],
    "mk/phony.mk": ["raft-ingress-production-guard-check"],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster.rs": 300,
    "crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs": 280,
    "scripts/raft_ingress_production_guard_check.py": 160,
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
        "schema_version": "cortexdb.raft_ingress_production_guard_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "server status can expose an explicitly configured multi-node ClusterConfig",
            "sync and Axum HTTP surfaces fail closed when configured primary context ingress is unavailable",
            "non-primary context ingress returns service_unavailable instead of silently serving local-only data",
            "single-node default cluster status remains compatible",
        ],
        "does_not_prove": [
            "leader election, load-balancer ingress, or linearizable arbitrary-node routing",
            "linearizable multi-node reads through production routing",
            "external witnessed transparency or KMS/HSM custody",
            "N-run soak stability for release-lane consensus promotion",
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
    print(f"Raft ingress production guard check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
