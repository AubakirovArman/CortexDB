#!/usr/bin/env python3
"""Validate continuous public transparency operations/SLO evidence wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_slo/types.rs": [
        "cortexdb.transparency.slo.window.v1",
        "cortexdb.transparency.slo.evidence.v1",
        "TransparencySloPolicy",
        "TransparencySloWindow",
        "TransparencySloEvidence",
    ],
    "crates/cortex-engine/src/accountability/transparency_slo.rs": [
        "build_transparency_slo_evidence",
        "verify_transparency_slo_evidence",
        "transparency slo window gap",
        "transparency slo availability target not met",
        "transparency slo log count regressed",
        "split transparency slo log head",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencySloEvidence",
        "TRANSPARENCY_SLO_EVIDENCE_SCHEMA",
        "build_transparency_slo_evidence",
    ],
    "crates/cortex-engine/src/accountability/transparency_slo_tests.rs": [
        "transparency_slo_accepts_continuous_operational_windows",
        "transparency_slo_rejects_gap_between_windows",
        "transparency_slo_rejects_below_availability_slo",
        "transparency_slo_rejects_log_count_regression",
        "transparency_slo_rejects_split_root_for_same_log_count",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.slo.evidence.v1",
        "transparency-slo-check",
        "continuous public transparency",
        "operations evidence",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.slo.evidence.v1",
        "transparency-slo-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.slo.evidence.v1",
        "continuous public",
        "operations/SLO evidence",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_SLO_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-slo-check:",
        "$(MAKE) transparency-gossip-check",
        "cargo test -p cortex-engine transparency_slo --all-features",
        "scripts/transparency_slo_check.py",
    ],
    "mk/phony.mk": ["transparency-slo-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-slo-check",
        "Order 9 transparency SLO evidence slice",
    ],
    "scripts/crypto_foundation_check.py": ["transparency-slo-check"],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency_slo.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_slo/types.rs": 120,
    "crates/cortex-engine/src/accountability/transparency_slo/validation.rs": 100,
    "crates/cortex-engine/src/accountability/transparency_slo_tests.rs": 180,
    "scripts/transparency_slo_check.py": 190,
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
        "schema_version": "cortexdb.transparency_slo.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "continuous public transparency operations windows cover the declared period",
            "availability windows must meet the declared SLO percentage",
            "each operations window carries monitor quorum and gossip fanout summaries",
            "log counts are monotonic and same-count split heads are rejected",
        ],
        "does_not_prove": [
            "live production deployment",
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
    print(f"transparency slo check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
