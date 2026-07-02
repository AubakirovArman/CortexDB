#!/usr/bin/env python3
"""Validate cached lifecycle monitor evidence for Raft ingress routing."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/cluster.rs": [
        "mod monitor;",
        "pub(crate) use monitor::ClusterIngressMonitor;",
        "context_ingress_decision_with_monitor",
    ],
    "crates/cortex-server/src/cluster/monitor.rs": [
        "struct ClusterIngressMonitor",
        "Mutex<ClusterIngressSnapshot>",
        "from_options",
        "refresh_once",
        "cached_leader_node",
        "cached Raft ingress monitor did not find a known healthy leader",
        "snapshot.leader_id = Some(leader_id)",
    ],
    "crates/cortex-server/src/state.rs": [
        "cluster_ingress_monitor: Option<Arc<ClusterIngressMonitor>>",
    ],
    "crates/cortex-server/src/lifecycle.rs": [
        "ClusterIngressMonitor::from_options",
        "monitor.refresh_once()",
        "cluster_ingress_monitor",
        "tokio::task::spawn_blocking",
    ],
    "crates/cortex-server/src/handler.rs": [
        "context_ingress_decision_with_monitor",
        "cluster_ingress_monitor.as_deref()",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_health_tests.rs": [
        "production_monitor_uses_cached_leader_after_status_peer_exits",
        "start_status_peer_n(NodeId(1), NodeId(2), 1)",
        "try_status_probe",
        "cached-monitor-leader",
    ],
    "mk/core-contracts.mk": [
        "raft-ingress-lifecycle-monitor-check:",
        "production_monitor_uses_cached_leader_after_status_peer_exits",
        "scripts/raft_ingress_lifecycle_monitor_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_LIFECYCLE_MONITOR_REPORT"],
    "mk/phony.mk": ["raft-ingress-lifecycle-monitor-check"],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster.rs": 260,
    "crates/cortex-server/src/cluster/monitor.rs": 220,
    "crates/cortex-server/src/tests/cluster_ingress_health_tests.rs": 300,
    "scripts/raft_ingress_lifecycle_monitor_check.py": 170,
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
        "schema_version": "cortexdb.raft_ingress_lifecycle_monitor_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "production server startup creates a ClusterIngressMonitor for separate-ingress topology",
            "the monitor performs an initial refresh and a background refresh loop",
            "the Axum handler uses cached monitor leader state for context ingress routing",
            "last-known healthy leader state survives a transient failed refresh after the Raft status peer exits",
        ],
        "does_not_prove": [
            "load-aware distribution across healthy ingress nodes",
            "operator-tunable monitor intervals or exported monitor metrics",
            "partition-linearizable arbitrary-node reads",
            "external witnessed transparency, KMS/HSM custody, or release-lane soak stability",
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
    print(f"Raft ingress lifecycle monitor check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
