#!/usr/bin/env python3
"""Validate independent witness quorum evidence for receipt transparency."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_quorum.rs": [
        "cortexdb.transparency.witness.quorum.v1",
        "TransparencyWitnessQuorumEvidence",
        "verify_transparency_witness_quorum",
        "transparency_witness_quorum_hash",
        "duplicate {name}",
        "mismatched witnessed log heads",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencyWitnessQuorumEvidence",
        "verify_transparency_witness_quorum",
        "TRANSPARENCY_WITNESS_QUORUM_SCHEMA",
    ],
    "crates/cortex-engine/src/accountability/transparency_quorum_tests.rs": [
        "transparency_witness_quorum_accepts_independent_log_head_witnesses",
        "transparency_witness_quorum_rejects_duplicate_public_key",
        "transparency_witness_quorum_rejects_split_log_heads",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.witness.quorum.v1",
        "transparency-witness-quorum-check",
        "witness quorum",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.witness.quorum.v1",
        "transparency-witness-quorum-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.witness.quorum.v1",
        "quorum evidence",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_WITNESS_QUORUM_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-witness-quorum-check:",
        "$(MAKE) transparency-witness-check",
        "cargo test -p cortex-engine transparency_witness_quorum --all-features",
        "scripts/transparency_witness_quorum_check.py",
    ],
    "mk/phony.mk": ["transparency-witness-quorum-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-witness-quorum-check",
        "Order 9 transparency witness quorum slice",
    ],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency_witness.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_quorum.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_quorum_tests.rs": 260,
    "scripts/transparency_witness_quorum_check.py": 170,
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
        "schema_version": "cortexdb.transparency_witness_quorum.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "multiple independent witness records agree on one verified local transparency log head",
            "quorum evidence rejects duplicate witness ids, key ids, and public keys",
            "quorum evidence rejects split log heads across individually valid witness records",
            "the quorum gate composes the existing transparency-witness-check",
        ],
        "does_not_prove": [
            "public transparency service availability",
            "CT-style gossip protocol or public log inclusion proofs",
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
    print(f"transparency witness quorum check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
