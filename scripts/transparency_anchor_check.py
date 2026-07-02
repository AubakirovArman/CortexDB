#!/usr/bin/env python3
"""Validate local receipt transparency anchor wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-engine/src/accountability/transparency.rs": [
        "cortexdb.transparency.log.record.v1",
        "cortexdb.transparency.log.record_hash.v1",
        "append_transparency_log_record",
        "read_transparency_log_records",
        "transparency log detected equivocation for determinism_hash",
        "pack_root",
        "determinism_hash",
        "receipt_signature_hex",
        "file.sync_data()",
    ],
    "crates/cortex-engine/src/accountability/transparency_tests.rs": [
        "transparency_log_appends_pack_root_chain",
        "transparency_log_rejects_equivocation_for_same_determinism_hash",
        "transparency_log_detects_record_tampering",
    ],
    "crates/cortex-server/src/receipt.rs": [
        "CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE",
        "transparency_log_path_from_env",
        "append_transparency_log_record",
        "requires configured receipt signing",
        "parse_transparency_log_path_rejects_empty_value",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE",
        "cortexdb.transparency.log.record.v1",
        "same determinism_hash",
    ],
    "docs/spec/RECEIPT_VERIFIER.md": [
        "transparency-anchor-check",
        "cortexdb.transparency.log.record.v1",
    ],
    "docs/SECURITY_MODEL.md": [
        "CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE",
        "local transparency log",
        "not a third-party witness",
    ],
    "mk/vars-core.mk": [
        "TRANSPARENCY_ANCHOR_REPORT ?= target/transparency-anchor/report.json",
    ],
    "mk/core-contracts.mk": [
        "transparency-anchor-check:",
        "cargo test -p cortex-engine transparency_log --all-features",
        "cargo test -p cortex-server parse_transparency_log_path",
        "python3 scripts/transparency_anchor_check.py --root \".\" --report \"$(TRANSPARENCY_ANCHOR_REPORT)\"",
    ],
    "mk/phony.mk": [
        "transparency-anchor-check",
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def build_report(root: Path) -> dict[str, object]:
    failures: list[str] = []
    checked: dict[str, list[str]] = {}
    for relative, markers in REQUIRED_MARKERS.items():
        text = read(root / relative)
        checked[relative] = markers
        for marker in markers:
            if marker not in text:
                failures.append(f"{relative}: missing {marker}")

    return {
        "schema_version": "cortexdb.transparency_anchor.report.v1",
        "status": "passed" if not failures else "failed",
        "checked": checked,
        "boundary": {
            "proves": "local append-only pack_root transparency log records are chained, tamper-detected, and reject same-determinism_hash equivocation when configured",
            "does_not_prove": "external witness availability, KMS/HSM custody, Byzantine transparency, or compliance-grade immutability",
        },
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    root = Path(args.root).resolve()
    try:
        report = build_report(root)
    except RuntimeError as error:
        print(f"transparency anchor check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"transparency anchor check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
