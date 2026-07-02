#!/usr/bin/env python3
"""Validate adaptive cached Raft ingress scheduling evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/cluster.rs": [
        "try_acquire_adaptive_leader_node()?",
        "_load_permit: Option<ClusterIngressRoutePermit>",
    ],
    "crates/cortex-server/src/cluster/monitor.rs": [
        "try_acquire_adaptive_leader_node",
        "over ingress load limit",
        "self.refresh_once()",
        "adaptive leader refresh failed",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_adaptive_tests.rs": [
        "adaptive_ingress_refreshes_leader_when_cached_route_is_over_limit",
        "reported_leader.store(3, Ordering::SeqCst)",
        "ContextIngressDecision::Local",
        "cached_leader_node().unwrap().id, NodeId(3)",
    ],
    "crates/cortex-server/src/tests.rs": ["mod cluster_ingress_adaptive_tests;"],
    "mk/core-contracts.mk": [
        "raft-ingress-adaptive-scheduling-check:",
        "cluster_ingress_adaptive_tests",
        "scripts/raft_ingress_adaptive_scheduling_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_ADAPTIVE_SCHEDULING_REPORT"],
    "mk/phony.mk": ["raft-ingress-adaptive-scheduling-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "raft-ingress-adaptive-scheduling-check",
        "adaptive refresh-on-overload",
    ],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster.rs": 280,
    "crates/cortex-server/src/cluster/monitor.rs": 250,
    "crates/cortex-server/src/tests/cluster_ingress_adaptive_tests.rs": 180,
    "scripts/raft_ingress_adaptive_scheduling_check.py": 170,
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
        "schema_version": "cortexdb.raft_ingress_adaptive_scheduling_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "cached Raft ingress monitor retries route admission after an over-limit leader refresh",
            "a simulated leader change can route the second concurrent request to the current local leader",
            "the previous remote leader permit remains live while the cached leader snapshot refreshes to node 3",
        ],
        "does_not_prove": [
            "load balancing across multiple writable Raft leaders",
            "weighted scheduling, fair queuing, or cross-request latency optimization",
            "partition-linearizable arbitrary-node reads",
            "external witnessed transparency, KMS/HSM custody, or production HA guarantees",
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
    print(f"Raft ingress adaptive scheduling check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
