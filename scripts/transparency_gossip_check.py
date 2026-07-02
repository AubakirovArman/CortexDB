#!/usr/bin/env python3
"""Validate public transparency gossip fanout evidence wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_gossip/types.rs": [
        "cortexdb.transparency.gossip.exchange.v1",
        "cortexdb.transparency.gossip.evidence.v1",
        "TransparencyGossipExchange",
        "TransparencyGossipPolicy",
        "TransparencyGossipEvidence",
    ],
    "crates/cortex-engine/src/accountability/transparency_gossip.rs": [
        "build_transparency_gossip_evidence",
        "verify_transparency_gossip_evidence",
        "transparency gossip fanout not met",
        "stale transparency gossip exchange",
        "split transparency gossip log head",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencyGossipEvidence",
        "TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA",
        "build_transparency_gossip_evidence",
    ],
    "crates/cortex-engine/src/accountability/transparency_gossip_tests.rs": [
        "transparency_gossip_accepts_required_monitor_fanout",
        "transparency_gossip_rejects_insufficient_fanout",
        "transparency_gossip_rejects_stale_exchange",
        "transparency_gossip_rejects_split_log_head",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.gossip.evidence.v1",
        "transparency-gossip-check",
        "network gossip fanout",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.gossip.evidence.v1",
        "transparency-gossip-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.gossip.evidence.v1",
        "continuous production SLO",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_GOSSIP_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-gossip-check:",
        "$(MAKE) transparency-availability-check",
        "cargo test -p cortex-engine transparency_gossip --all-features",
        "scripts/transparency_gossip_check.py",
    ],
    "mk/phony.mk": ["transparency-gossip-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-gossip-check",
        "Order 9 transparency gossip fanout slice",
    ],
    "scripts/crypto_foundation_check.py": ["transparency-gossip-check"],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency_gossip.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_gossip/types.rs": 120,
    "crates/cortex-engine/src/accountability/transparency_gossip_tests.rs": 180,
    "scripts/transparency_gossip_check.py": 180,
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
        "schema_version": "cortexdb.transparency_gossip.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "fresh monitor-to-monitor gossip exchanges can be fanout-gated",
            "each participating monitor must send to the required number of peers",
            "stale gossip exchanges are rejected",
            "split transparency log heads are rejected",
        ],
        "does_not_prove": [
            "continuous production SLO compliance",
            "Byzantine monitor key custody",
            "KMS/HSM custody or compliance-grade immutability",
            "release-lane soak stability",
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
    print(f"transparency gossip check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
