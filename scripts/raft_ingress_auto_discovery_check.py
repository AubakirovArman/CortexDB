#!/usr/bin/env python3
"""Validate the Raft STATUS based ingress leader discovery evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/distributed.rs": [
        "pub ingress_address: Option<String>",
        "pub fn ingress_address(&self) -> &str",
        '"node {} {} {}\\n"',
    ],
    "crates/cortex-engine/src/replication/tcp.rs": [
        '["STATUS"] => Ok(status_response(state))',
        '"STATUS_RESP {} {} {} {}\\n"',
        "ElectionRole::Leader",
    ],
    "crates/cortex-engine/tests/replication_cluster_config.rs": [
        "cluster_config_roundtrips_optional_ingress_addresses",
        "ingress_address()",
    ],
    "crates/cortex-engine/tests/replication_transport.rs": [
        "replication_status_frame_reports_known_leader_without_log_mutation",
        "STATUS_RESP 2 2 follower 1",
    ],
    "crates/cortex-server/src/cluster.rs": [
        "cluster_uses_separate_ingress",
        "discover_raft_leader_node",
        "context_ingress_decision_with_monitor",
        "automatic Raft ingress leader discovery did not find a known leader",
        "leader.node.ingress_address().to_owned()",
    ],
    "crates/cortex-server/src/cluster/monitor.rs": [
        "request_raft_status_leader",
        "parse_raft_status_leader",
        "discover_raft_leader_node",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_discovery_tests.rs": [
        "context_route_discovers_raft_leader_then_forwards_to_ingress_address",
        "separate_ingress_config_fails_closed_when_raft_leader_discovery_is_unavailable",
        "start_status_peer",
        "raft-ingress-discovery-test",
    ],
    "crates/cortex-server/src/tests.rs": ["mod cluster_ingress_discovery_tests;"],
    "mk/core-contracts.mk": [
        "raft-ingress-auto-discovery-check:",
        "replication_status_frame_reports_known_leader_without_log_mutation",
        "cluster_ingress_discovery_tests",
        "scripts/raft_ingress_auto_discovery_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_AUTO_DISCOVERY_REPORT"],
    "mk/phony.mk": ["raft-ingress-auto-discovery-check"],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster.rs": 300,
    "crates/cortex-server/src/cluster/monitor.rs": 220,
    "crates/cortex-server/src/tests/cluster_ingress_discovery_tests.rs": 280,
    "scripts/raft_ingress_auto_discovery_check.py": 180,
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
        "schema_version": "cortexdb.raft_ingress_auto_discovery_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "cluster_config can model separate Raft peer and HTTP ingress addresses while preserving old two-column node records",
            "Raft peers expose a read-only STATUS frame that reports current term, local node, role, and known leader",
            "context ingress can query Raft STATUS, discover a known leader, and forward to the leader HTTP ingress address",
            "separate-ingress topology fails closed when automatic Raft leader discovery cannot find a known leader",
        ],
        "does_not_prove": [
            "load-aware or health-aware balancing across multiple healthy leaders",
            "production lifecycle wiring to a long-running cluster manager or cached peer-status monitor",
            "linearizable arbitrary-node reads through leadership changes and partitions",
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
    print(f"Raft ingress auto-discovery check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
