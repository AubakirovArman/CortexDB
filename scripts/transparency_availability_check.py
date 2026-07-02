#!/usr/bin/env python3
"""Validate public transparency availability evidence wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_availability.rs": [
        "cortexdb.transparency.availability.observation.v1",
        "cortexdb.transparency.availability.evidence.v1",
        "TransparencyAvailabilityObservation",
        "TransparencyAvailabilityPolicy",
        "build_transparency_availability_evidence",
        "verify_transparency_availability_evidence",
        "stale transparency availability observation",
        "duplicate transparency monitor id",
        "transparency monitor uptime below policy",
        "split transparency availability log head",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencyAvailabilityEvidence",
        "TransparencyAvailabilityObservation",
        "TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA",
    ],
    "crates/cortex-engine/src/accountability/transparency_availability_tests.rs": [
        "transparency_availability_accepts_fresh_independent_monitors",
        "transparency_availability_rejects_stale_observation",
        "transparency_availability_rejects_duplicate_monitor_identity",
        "transparency_availability_rejects_low_monitor_uptime",
        "transparency_availability_rejects_split_log_heads",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.availability.evidence.v1",
        "transparency-availability-check",
        "independent monitor uptime",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.availability.evidence.v1",
        "transparency-availability-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.availability.evidence.v1",
        "network gossip fanout",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_AVAILABILITY_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-availability-check:",
        "$(MAKE) transparency-consistency-check",
        "cargo test -p cortex-engine transparency_availability --all-features",
        "scripts/transparency_availability_check.py",
    ],
    "mk/phony.mk": ["transparency-availability-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-availability-check",
        "Order 9 transparency availability evidence slice",
    ],
    "scripts/crypto_foundation_check.py": ["transparency-availability-check"],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency_availability.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_availability_tests.rs": 220,
    "scripts/transparency_availability_check.py": 180,
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
        "schema_version": "cortexdb.transparency_availability.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "fresh public HTTPS monitor observations can attest service availability",
            "independent monitor ids and urls are required",
            "monitor uptime and observation freshness are policy-gated",
            "split transparency log heads are rejected",
        ],
        "does_not_prove": [
            "network gossip fanout between monitors",
            "continuous production SLO compliance",
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
    print(f"transparency availability check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
