#!/usr/bin/env python3
"""Validate CT-style inclusion proof evidence for receipt transparency."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_inclusion.rs": [
        "cortexdb.transparency.inclusion.proof.v1",
        "TransparencyInclusionProof",
        "TransparencyInclusionSibling",
        "build_transparency_inclusion_proof",
        "verify_transparency_inclusion_proof",
        "transparency_inclusion_root_hash",
        "transparency inclusion root hash mismatch",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencyInclusionProof",
        "build_transparency_inclusion_proof",
        "TRANSPARENCY_INCLUSION_PROOF_SCHEMA",
    ],
    "crates/cortex-engine/src/accountability/transparency_inclusion_tests.rs": [
        "transparency_inclusion_proof_accepts_middle_record",
        "transparency_inclusion_proof_rejects_wrong_record_hash",
        "transparency_inclusion_proof_rejects_wrong_path_hash",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.inclusion.proof.v1",
        "transparency-inclusion-check",
        "Merkle inclusion proof",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.inclusion.proof.v1",
        "transparency-inclusion-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.inclusion.proof.v1",
        "inclusion proof",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_INCLUSION_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-inclusion-check:",
        "$(MAKE) transparency-witness-quorum-check",
        "cargo test -p cortex-engine transparency_inclusion --all-features",
        "scripts/transparency_inclusion_check.py",
    ],
    "mk/phony.mk": ["transparency-inclusion-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-inclusion-check",
        "Order 9 transparency inclusion proof slice",
    ],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency_inclusion.rs": 260,
    "crates/cortex-engine/src/accountability/transparency_inclusion_tests.rs": 180,
    "scripts/transparency_inclusion_check.py": 170,
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
        "schema_version": "cortexdb.transparency_inclusion.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "local transparency records can carry a Merkle inclusion proof",
            "proof verification rejects record-hash tampering",
            "proof verification rejects sibling-path tampering",
            "the inclusion gate composes the existing witness quorum gate",
        ],
        "does_not_prove": [
            "public transparency service availability",
            "CT-style gossip or consistency exchange between public monitors",
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
    print(f"transparency inclusion check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
