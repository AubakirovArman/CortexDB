#!/usr/bin/env python3
"""Validate the multi-agent cluster consistency evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/tests/multi_agent_cluster_consistency.rs": [
        "read_your_writes_survives_follower_read_and_leader_failover",
        "monotonic_read_and_handoff_survive_partition_heal",
        "MemoryConsistencyLevel::SharedImmediate",
        "MemoryConsistencyLevel::SharedSequenced",
        "plan_agent_handoff",
        "set_partitions",
        "heal_partitions",
    ],
    "crates/cortex-engine/tests/multi_agent_cluster_consistency/support.rs": [
        "AgentTransactionOptions",
        "commit_agent_transaction",
        "ReplicationPeerServer",
        "TcpReplicationTransport",
        "InMemoryReplicationTransport",
        "current_seq",
        "retrieve_aql",
        "AgentHandoffRequest",
    ],
    "mk/core-contracts.mk": [
        "multi-agent-cluster-consistency-check:",
        "cargo test -p cortex-engine --test multi_agent_cluster_consistency --all-features",
        "scripts/multi_agent_cluster_consistency_check.py",
    ],
    "mk/vars-core.mk": ["MULTI_AGENT_CLUSTER_CONSISTENCY_REPORT"],
    "mk/phony.mk": ["multi-agent-cluster-consistency-check"],
}

LINE_LIMITS = {
    "crates/cortex-engine/tests/multi_agent_cluster_consistency.rs": 300,
    "crates/cortex-engine/tests/multi_agent_cluster_consistency/support.rs": 300,
    "scripts/multi_agent_cluster_consistency_check.py": 160,
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
        "schema_version": "cortexdb.multi_agent_cluster_consistency_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "read-your-writes survives leader write, follower read, and follower promotion",
            "monotonic reads advance from last_seen_seq after partition heal",
            "stale follower rejects future SharedSequenced handoff before catch-up",
            "healed follower accepts SharedSequenced handoff after replicated catch-up",
            "cluster scenarios use declared SharedImmediate and SharedSequenced consistency levels",
        ],
        "does_not_prove": [
            "full HTTP/Raft arbitrary-node request routing",
            "external witnessed transparency or KMS/HSM custody",
            "N-run soak stability for release-lane promotion",
            "linearizable reads without explicit catch-up to the committed sequence",
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
    print(f"multi-agent cluster consistency check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
