#!/usr/bin/env python3
"""Validate the fixed-primary live Raft ingress forwarding evidence gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-server/src/cluster.rs": [
        "ContextIngressDecision::Forward",
        "ForwardTarget",
        "forward_http_request",
        "parse_forwarded_response",
        "Connection: close",
    ],
    "crates/cortex-server/src/handler.rs": [
        "tokio::task::spawn_blocking",
        "crate::cluster::forward_http_request",
        "forwarded_body_response",
        "accountability_receipt_audit_hash_from_response_body",
    ],
    "crates/cortex-server/src/sync_handler.rs": [
        "crate::cluster::forward_http_request",
        "json_response(response.status_code, &response.body)",
    ],
    "crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs": [
        "non_primary_context_route_forwards_to_live_primary",
        "serve_with_options",
        "request_full(follower_addr",
        'value["accountability_receipt"]',
        '"raft-ingress-forwarding-test"',
    ],
    "mk/core-contracts.mk": [
        "raft-ingress-forwarding-check:",
        "cargo test -p cortex-server cluster_ingress_guard_tests --all-features",
        "scripts/raft_ingress_forwarding_check.py",
    ],
    "mk/vars-core.mk": ["RAFT_INGRESS_FORWARDING_REPORT"],
    "mk/phony.mk": ["raft-ingress-forwarding-check"],
}

LINE_LIMITS = {
    "crates/cortex-server/src/cluster.rs": 300,
    "crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs": 280,
    "scripts/raft_ingress_forwarding_check.py": 160,
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
        "schema_version": "cortexdb.raft_ingress_forwarding_gate.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "non-primary Axum context ingress can forward a live HTTP request to the configured primary node",
            "forwarded context responses preserve JSON bodies and signed accountability receipt payloads",
            "primary-unavailable non-primary ingress remains service_unavailable without local fallback",
            "the legacy sync harness uses the same fixed-primary forwarding helper",
        ],
        "does_not_prove": [
            "automatic Raft leader discovery or load-balancer routing",
            "linearizable reads from arbitrary live nodes during leadership changes",
            "external witnessed transparency, KMS/HSM custody, or compliance immutability",
            "release-lane consensus soak stability",
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
    print(f"Raft ingress forwarding check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
