#!/usr/bin/env python3
"""Validate external mirror witness evidence for receipt transparency."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency_witness.rs": [
        "cortexdb.transparency.witness.record.v1",
        "cortexdb.transparency.witness.sign.v1",
        "TransparencyWitnessRecord",
        "TransparencyWitnessSigningKey",
        "witness_transparency_log",
        "verify_transparency_witness_record",
        "log_head_hash",
        "witness_signature_hex",
        "ed25519_verify",
    ],
    "crates/cortex-engine/src/accountability.rs": [
        "TransparencyWitnessRecord",
        "TransparencyWitnessSigningKey",
        "witness_transparency_log",
        "TRANSPARENCY_WITNESS_RECORD_SCHEMA",
    ],
    "crates/cortex-engine/src/accountability/transparency_tests.rs": [
        "transparency_witness_signs_log_head_and_verifies",
        "transparency_witness_detects_tampered_head_after_hash_recompute",
        "verify_transparency_witness_record",
        "transparency_witness_record_hash",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "cortexdb.transparency.witness.record.v1",
        "transparency-witness-check",
        "external mirror witness",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "cortexdb.transparency.witness.record.v1",
        "transparency-witness-check",
    ],
    "docs/SECURITY_MODEL.md": [
        "cortexdb.transparency.witness.record.v1",
        "external mirror",
    ],
    "mk/vars-core.mk": ["TRANSPARENCY_WITNESS_REPORT"],
    "mk/core-contracts.mk": [
        "transparency-witness-check:",
        "$(MAKE) transparency-anchor-check",
        "cargo test -p cortex-engine transparency_witness --all-features",
        "scripts/transparency_witness_check.py",
    ],
    "mk/phony.mk": ["transparency-witness-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "transparency-witness-check",
        "Order 9 external transparency witness mirror slice",
    ],
}

LINE_LIMITS = {
    "crates/cortex-engine/src/accountability/transparency.rs": 260,
    "crates/cortex-engine/src/accountability/transparency_witness.rs": 300,
    "crates/cortex-engine/src/accountability/transparency_tests.rs": 260,
    "scripts/transparency_witness_check.py": 170,
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
        "schema_version": "cortexdb.transparency_witness.report.v1",
        "status": "passed",
        "checked_files": checked,
        "line_counts": line_counts,
        "proves": [
            "a local transparency log head can be mirrored into a separate witness record",
            "the witness record signs the local log head and sequence range with an independent Ed25519 key",
            "verification rejects body tampering even when the witness record hash is recomputed",
            "the witness gate composes the existing local transparency-anchor-check",
        ],
        "does_not_prove": [
            "public transparency service availability",
            "CT-style gossip or Byzantine witness quorum",
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
    print(f"transparency witness check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
