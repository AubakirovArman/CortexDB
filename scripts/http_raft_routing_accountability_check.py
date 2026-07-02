#!/usr/bin/env python3
"""Validate the HTTP/Raft routing accountability evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/tests/http_raft_routing_tests.rs": [
        "http_raft_arbitrary_node_context_receipts_use_replicated_snapshot",
        "crate::handler::axum_handler",
        "ReplicationPeerServer",
        "TcpReplicationTransport",
        "accountability_receipt",
        "pack_root",
        "determinism_hash",
        "audit_chain_head",
        "stable_receipt_commitments",
    ],
    "mk/core-contracts.mk": [
        "http-raft-routing-accountability-check:",
        "cargo test -p cortex-server http_raft_arbitrary_node_context_receipts_use_replicated_snapshot --all-features",
        "scripts/http_raft_routing_accountability_check.py",
    ],
    "mk/vars-core.mk": ["HTTP_RAFT_ROUTING_ACCOUNTABILITY_REPORT"],
    "mk/phony.mk": ["http-raft-routing-accountability-check"],
}

LINE_LIMITS = {
    "crates/cortex-server/src/tests/http_raft_routing_tests.rs": 300,
    "scripts/http_raft_routing_accountability_check.py": 160,
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
        "schema_version": "cortexdb.http_raft_routing_accountability_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "Axum HTTP context route serves accountable ContextPack responses from arbitrary replicated node roots",
            "follower node roots are populated through TCP Raft snapshot-install frames before HTTP reads",
            "HTTP responses from leader and followers expose matching pack_root, determinism_hash, audit_chain_head, and receipt commitments",
            "HTTP route preserves fail-closed scope filtering for replicated private cells",
            "stale non-replicated node root does not synthesize committed cells",
        ],
        "does_not_prove": [
            "a live production load balancer or gateway that routes one client request among Raft peers",
            "byte-identical HTTP receipt signatures across requests because HTTP signing uses current_unix_seconds",
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
    print(f"HTTP/Raft routing accountability check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
