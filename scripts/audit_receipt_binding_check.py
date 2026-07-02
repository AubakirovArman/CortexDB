#!/usr/bin/env python3
"""Validate CRY-5 audit binding for emitted accountability receipt hashes."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REQUIRED_MARKERS = {
    "crates/cortex-server/src/audit.rs": [
        "accountability_receipt_hash",
        "audit_event_fields",
    ],
    "crates/cortex-server/src/receipt.rs": [
        "ACCOUNTABILITY_RECEIPT_AUDIT_HASH_DOMAIN",
        "accountability_receipt_audit_hash",
        "accountability_receipt_audit_hash_from_response_body",
    ],
    "crates/cortex-server/src/request_audit.rs": [
        "audit_http_response_with_receipt_hash",
        "accountability_receipt_hash: Option<&str>",
    ],
    "crates/cortex-server/src/handler/database_route.rs": [
        "accountability_receipt_audit_hash_from_response_body",
        "audit_http_response_with_receipt_hash",
    ],
    "crates/cortex-cli/src/cli_audit.rs": [
        "accountability_receipt_hash",
    ],
    "crates/cortex-cli/src/cli_audit_chain.rs": [
        "accountability_receipt_hash",
    ],
    "crates/cortex-server/src/audit_tests.rs": [
        "audit_sink_records_accountability_receipt_hash",
    ],
    "crates/cortex-server/src/tests/security_tests/audit.rs": [
        "audit_log_binds_emitted_context_receipt_hash",
    ],
    "crates/cortex-cli/src/cli_audit_tests.rs": [
        "audit_review_verify_chain_rejects_receipt_hash_tampering",
    ],
    "docs/AUDIT_LOG_FORMAT.md": [
        "accountability_receipt_hash",
        "included in `event_hash` and `event_mac`",
    ],
    "docs/AUTH.md": [
        "accountability_receipt_hash",
        "audit records commit the receipt hash",
    ],
    "docs/SECURITY_MODEL.md": [
        "accountability_receipt_hash",
    ],
    "mk/core-security-ops.mk": [
        "audit-receipt-binding-check:",
        "cargo test -p cortex-server receipt_hash",
        "cargo test -p cortex-cli audit_review_verify_chain_rejects_receipt_hash_tampering",
        'python3 scripts/audit_receipt_binding_check.py --root "." --report "$(AUDIT_RECEIPT_BINDING_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "AUDIT_RECEIPT_BINDING_REPORT ?= target/audit-receipt-binding/report.json",
    ],
    "mk/phony.mk": [
        "audit-receipt-binding-check",
    ],
}


def read_text(root: Path, relative: str) -> str:
    path = root / relative
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {relative}: {error}") from error


def contains_marker(text: str, marker: str) -> bool:
    if marker in text:
        return True
    return " ".join(marker.split()) in " ".join(text.split())


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, list[str]] = {}

    for relative, markers in REQUIRED_MARKERS.items():
        text = read_text(root, relative)
        checked[relative] = markers
        for marker in markers:
            if not contains_marker(text, marker):
                failures.append(f"{relative}: missing marker {marker!r}")

    return {
        "schema_version": "cortexdb.audit_receipt_binding.report.v1",
        "status": "passed" if not failures else "failed",
        "production_safe": not failures,
        "checked": checked,
        "failures": failures,
        "boundary": {
            "proves": "emitted accountability receipt hashes are committed into local audit v2 records and verified by the audit chain/MAC verifier",
            "does_not_prove": "receipt/audit re-anchor records, transparency log anchoring, KMS/HSM custody, durable database-instance identity, or compliance-grade immutability",
        },
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(Path(args.root).resolve())
    except RuntimeError as error:
        print(f"audit receipt binding check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"audit receipt binding check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
