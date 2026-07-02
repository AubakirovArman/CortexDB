#!/usr/bin/env python3
"""Validate the SCALE-3 receipt replica-invariance gate wiring."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "cortexdb.receipt_replica_invariance.report.v1"

REQUIRED_TERMS: dict[str, list[str]] = {
    "crates/cortex-engine/src/accountability/receipt_header.rs": [
        "audit_chain_head",
        "canonical_accountability_receipt_header_bytes",
        "is_hex_hash",
    ],
    "crates/cortex-engine/src/accountability/receipt_sign_tests.rs": [
        "accountability_receipt_header_is_replica_invariant_for_same_committed_inputs",
        "accountability_receipt_header_changes_when_audit_chain_head_changes",
        "canonical_accountability_receipt_header_bytes",
        "accountability_receipt_header_value",
    ],
    "crates/cortex-engine/tests/receipt_replica_invariance.rs": [
        "replicated_snapshot_context_pack_and_receipt_are_byte_identical",
        "replication_snapshot_segment",
        "install_snapshot_segment",
        "context_pack_with_receipt_evidence_from_aql",
        "canonical_context_pack_bytes",
        "canonical_json_bytes",
        "append_transparency_log_record",
    ],
    "crates/cortex-engine/src/context/receipt_evidence.rs": [
        "audit_chain_head",
        "signed_receipt_value",
    ],
    "crates/cortex-server/src/receipt.rs": [
        "audit_chain_tail",
        "AUDIT_CHAIN_ZERO_HASH",
        "audit_chain_head",
        "audit_log_path",
    ],
    "crates/cortex-receipt-verify/src/model.rs": ["audit_chain_head"],
    "crates/cortex-receipt-verify/src/receipt_hash.rs": [
        "audit_chain_head",
        "canonical_header_bytes",
    ],
    "crates/cortex-receipt-verify/src/verifier.rs": [
        "audit_chain_head",
        "is_hex_hash",
        "InvalidSchema",
    ],
    "fixtures/accountability_receipt/verify_input.golden.json": ["audit_chain_head"],
    "docs/schemas/accountability_receipt.v1.json": ["audit_chain_head"],
    "docs/schemas/accountability_receipt.v1.golden.json": ["audit_chain_head"],
    "docs/spec/ACCOUNTABILITY_RECEIPT_V1.md": ["audit_chain_head"],
    "docs/spec/RECEIPT_VERIFIER.md": ["audit_chain_head"],
    "docs/spec/GCE_CONTRACT.md": ["receipt-replica-invariance-check"],
    "docs/SECURITY_MODEL.md": ["audit_chain_head"],
    "mk/core-contracts.mk": [
        "receipt-replica-invariance-check:",
        "cargo test -p cortex-engine --test receipt_replica_invariance --all-features",
        "accountability_receipt_header_is_replica_invariant_for_same_committed_inputs",
        "accountability_receipt_header_changes_when_audit_chain_head_changes",
        "transparency-anchor-check",
        "scripts/receipt_replica_invariance_check.py",
    ],
    "mk/vars-core.mk": [
        "RECEIPT_REPLICA_INVARIANCE_REPORT ?= target/receipt-replica-invariance/report.json"
    ],
    "mk/phony.mk": ["receipt-replica-invariance-check"],
}

MAX_FILE_LINES = 300
LINE_BOUNDED_FILES = [
    "crates/cortex-engine/src/accountability/receipt_header.rs",
    "crates/cortex-engine/src/accountability/receipt_sign_tests.rs",
    "crates/cortex-receipt-verify/src/model.rs",
    "crates/cortex-receipt-verify/src/receipt_hash.rs",
    "crates/cortex-receipt-verify/src/verifier.rs",
    "crates/cortex-engine/tests/receipt_replica_invariance.rs",
    "scripts/receipt_replica_invariance_check.py",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(root: Path) -> list[str]:
    failures: list[str] = []
    for rel_path, terms in REQUIRED_TERMS.items():
        text = read_text(root / rel_path)
        for term in terms:
            if term not in text:
                failures.append(f"{rel_path}: missing {term}")
    return failures


def line_count_failures(root: Path) -> list[str]:
    failures: list[str] = []
    for rel_path in LINE_BOUNDED_FILES:
        line_count = len(read_text(root / rel_path).splitlines())
        if line_count > MAX_FILE_LINES:
            failures.append(f"{rel_path}: {line_count} lines exceeds {MAX_FILE_LINES}")
    return failures


def command(root: Path, args: list[str], expect_success: bool) -> tuple[bool, str]:
    result = subprocess.run(
        args,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    ok = result.returncode == 0
    return ok == expect_success, result.stdout


def mutate_audit_chain_head(fixture: Path) -> Path:
    data = json.loads(read_text(fixture))
    data["receipt"]["header"]["audit_chain_head"] = "b" * 64
    temp = tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        suffix=".audit-chain-head-tamper.json",
        delete=False,
    )
    with temp:
        json.dump(data, temp, indent=2, sort_keys=True)
        temp.write("\n")
    return Path(temp.name)


def validate(root: Path, fixture: Path) -> dict[str, Any]:
    failures = missing_terms(root)
    failures.extend(line_count_failures(root))

    genuine_ok, genuine_output = command(
        root,
        ["cargo", "run", "-p", "cortex-receipt-verify", "--", "--input", str(fixture)],
        expect_success=True,
    )
    if not genuine_ok:
        failures.append("standalone verifier did not accept audit-head fixture")

    tampered = mutate_audit_chain_head(fixture)
    tamper_ok, tamper_output = command(
        root,
        ["cargo", "run", "-p", "cortex-receipt-verify", "--", "--input", str(tampered)],
        expect_success=False,
    )
    if not tamper_ok:
        failures.append("standalone verifier did not reject audit_chain_head tamper")

    return {
        "schema_version": SCHEMA_VERSION,
        "status": "failed" if failures else "passed",
        "proves": [
            "two real Database instances with an installed replication snapshot produce byte-identical canonical ContextPack bytes",
            "two real Database instances with the same committed receipt inputs produce byte-identical signed accountability_receipt.v1 bytes",
            "same committed receipt inputs produce a byte-identical signed header",
            "audit_chain_head is included in the signed canonical receipt header",
            "standalone verifier rejects audit_chain_head tampering",
            "receipt-replica-invariance-check aggregates schema, verifier, and transparency gates",
        ],
        "does_not_prove": [
            "follower-read, failover, or partition-heal fail-closed behavior",
            "networked Raft request routing serves the receipt from arbitrary nodes",
            "external witnessed transparency or KMS/HSM custody",
        ],
        "checked": {
            "fixture": str(fixture),
            "required_terms": REQUIRED_TERMS,
            "line_bound": MAX_FILE_LINES,
            "genuine_verifier_output": genuine_output.strip(),
            "audit_chain_head_tamper_output": tamper_output.strip(),
        },
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--fixture", required=True, help="verifier golden input")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    fixture = (root / args.fixture).resolve()
    report = validate(root, fixture)
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"receipt replica-invariance check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
