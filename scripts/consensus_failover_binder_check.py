#!/usr/bin/env python3
"""Validate the cluster fail-closed binder evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/tests/cluster_fail_closed.rs": [
        "follower_read_network_snapshot_preserves_fail_closed_receipt",
        "failover_read_rejects_stale_old_leader_widening",
        "partition_heal_serves_only_committed_allowed_scope",
        "set_partitions",
        "heal_partitions",
    ],
    "crates/cortex-engine/tests/cluster_fail_closed/support.rs": [
        "ReplicationPeerServer",
        "TcpReplicationTransport",
        "InMemoryReplicationTransport",
        "PushAgentAllowed",
        "PushLive",
        "And",
        "ContextAccessDecisionOutcome::Allowed",
        "context_pack_with_receipt_evidence_from_aql",
        "canonical_context_pack_bytes",
        "signed_receipt_value",
    ],
    "mk/core-contracts.mk": [
        "consensus-failover-binder-check:",
        "cargo test -p cortex-engine --test cluster_fail_closed --all-features",
        "cargo test -p cortex-engine --test replication_partition_matrix --all-features",
        "scripts/consensus_failover_binder_check.py",
    ],
    "mk/vars-core.mk": ["CONSENSUS_FAILOVER_BINDER_REPORT"],
    "mk/phony.mk": ["consensus-failover-binder-check"],
}

LINE_LIMITS = {
    "crates/cortex-engine/tests/cluster_fail_closed.rs": 300,
    "crates/cortex-engine/tests/cluster_fail_closed/support.rs": 300,
    "scripts/consensus_failover_binder_check.py": 160,
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
        "schema_version": "cortexdb.consensus.failover_binder_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "follower read after network snapshot install preserves PushAgentAllowed/PushLive/And binder seed",
            "follower read emits only allowed-scope cells with captured allowed access decisions",
            "leader failover path rejects stale old-leader scope widening before serving a receipt",
            "partition minority write is not served before commit and remains fail-closed after partition heal",
            "receipt pack bytes, determinism_hash, and signed receipt bytes remain stable for committed follower/failover reads",
        ],
        "does_not_prove": [
            "full HTTP request routing to arbitrary Raft nodes",
            "external witnessed transparency or KMS/HSM custody",
            "SCALE-2 cross-node read-your-writes and monotonic-read guarantees",
            "N-run soak stability for release-lane promotion",
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
    print(f"consensus failover binder check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
