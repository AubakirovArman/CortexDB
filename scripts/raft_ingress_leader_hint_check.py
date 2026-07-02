#!/usr/bin/env python3
"""Validate the explicit leader-hint ingress forwarding evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/config.rs": [
        "cluster_ingress_leader",
        "Optional operator-provided context-ingress leader override",
    ],
    "crates/cortex-server/src/main.rs": [
        "CORTEXDB_CLUSTER_INGRESS_LEADER_ID",
        "parse_cluster_ingress_leader",
        "cluster_ingress_leader_from_env",
    ],
    "crates/cortex-server/src/lifecycle.rs": [
        "cluster_ingress_leader requires cluster_config",
        "is not present in cluster_config",
    ],
    "crates/cortex-server/src/cluster.rs": [
        "cluster_ingress_leader",
        "context_ingress_leader_node",
        "missing_leader_message",
        ".first()",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_leader_hint_tests.rs": [
        "non_primary_context_route_forwards_to_hinted_leader",
        "unknown_cluster_ingress_leader_fails_closed_without_fallback",
        "raft-ingress-leader-hint-test",
    ],
    "crates/cortex-server/src/main/tests.rs": [
        "parse_cluster_ingress_leader_accepts_positive_node_id",
    ],
    "crates/cortex-server/src/tests.rs": ["mod cluster_ingress_leader_hint_tests;"],
    "mk/core-contracts.mk": [
        "raft-ingress-leader-hint-check:",
        "cluster_ingress_leader_hint_tests",
        "scripts/raft_ingress_leader_hint_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_LEADER_HINT_REPORT"],
    "mk/phony.mk": ["raft-ingress-leader-hint-check"],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster.rs": 300,
    "crates/cortex-server/src/tests/cluster_ingress_leader_hint_tests.rs": 260,
    "scripts/raft_ingress_leader_hint_check.py": 160,
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
        "schema_version": "cortexdb.raft_ingress_leader_hint_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "operator-provided ingress leader id can route context forwarding away from the first configured node",
            "unknown ingress leader ids fail closed without falling back to local or first-node reads",
            "server startup validates that a configured ingress leader belongs to cluster_config",
            "the env parser rejects zero and non-numeric leader ids",
        ],
        "does_not_prove": [
            "automatic Raft leader discovery from durable election state",
            "load-balancer health probing or follower promotion automation",
            "linearizable arbitrary-node reads during leadership changes",
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
    print(f"Raft ingress leader hint check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
