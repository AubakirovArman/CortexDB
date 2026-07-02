#!/usr/bin/env python3
"""Validate public-monitor transparency consistency evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_consistency.rs": [
        "cortexdb.transparency.consistency.v1",
        "TransparencyConsistencyEvidence",
        "build_transparency_consistency_evidence",
        "verify_transparency_consistency_evidence",
        "transparency consistency divergent prefix",
        "transparency consistency new snapshot is shorter",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencyConsistencyEvidence",
        "build_transparency_consistency_evidence",
        "TRANSPARENCY_CONSISTENCY_SCHEMA",
    ],
    "crates/cortex-engine/src/accountability/transparency_consistency_tests.rs": [
        "transparency_consistency_accepts_append_only_snapshot",
        "transparency_consistency_rejects_divergent_prefix",
        "transparency_consistency_rejects_truncated_snapshot",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.consistency.v1",
        "transparency-consistency-check",
        "append-only consistency evidence",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.consistency.v1",
        "transparency-consistency-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.consistency.v1",
        "truncated newer snapshots",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_CONSISTENCY_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-consistency-check:",
        "$(MAKE) transparency-inclusion-check",
        "cargo test -p cortex-engine transparency_consistency --all-features",
        "scripts/transparency_consistency_check.py",
    ],
    "mk/phony.mk": ["transparency-consistency-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-consistency-check",
        "Order 9 transparency consistency evidence slice",
    ],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency_consistency.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_consistency_tests.rs": 180,
    "scripts/transparency_consistency_check.py": 170,
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
        "schema_version": "cortexdb.transparency_consistency.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "published transparency snapshots can be compared for append-only consistency",
            "divergent snapshot prefixes are rejected",
            "truncated newer snapshots are rejected",
            "the consistency gate composes the existing inclusion gate",
        ],
        "does_not_prove": [
            "public transparency service availability",
            "network gossip fanout or independent public monitor uptime",
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
    print(f"transparency consistency check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
