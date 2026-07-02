#!/usr/bin/env python3
"""Validate the receipt verifier threat-model specification."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "cortexdb.receipt_threat_model.report.v1"

DOC_MARKERS = [
    "# Accountability Receipt Verifier",
    "Status: normative verifier algorithm and threat model",
    "## Public Inputs",
    "## Seven Verifier Steps",
    "## Eight Forgery Classes",
    "## Gate Requirements",
    "## Out Of Scope",
    "cortexdb.accountability_receipt_verify_input.v1",
    "cortex-receipt-verify",
    "docs/schemas/accountability_receipt.v1.json",
    "docs/spec/GCE_CONTRACT.md",
]

STEP_MARKERS = [
    "Step 1 - Validate public input and header shape",
    "Step 2 - Verify Ed25519 signature over the canonical header",
    "Step 3 - Recompute all root commitments",
    "Step 4 - Enforce admitted-cell access leaves",
    "Step 5 - Check cell-set identity and provenance spans",
    "Step 6 - Check token budget consistency",
    "Step 7 - Check verification references and deterministic input binding",
]

VERIFIER_FUNCTION_MARKERS = [
    "verify_header_shape",
    "verify_signature",
    "verify_roots",
    "verify_access",
    "verify_cell_set",
    "verify_provenance",
    "verify_budget",
    "verify_verification_references",
]

DEFENDING_FIELDS = [
    "access_root",
    "provenance_root",
    "cell_set_root",
    "verification_root",
    "budget_commitment",
    "conflict_commitment",
    "pack_root",
    "determinism_hash",
    "audit_chain_head",
    "signature",
    "public_key",
    "cell_content_hash",
    "source_byte_start",
    "source_byte_end",
]

FORGERY_CLASSES = [
    "access_allowed_to_denied",
    "source_byte_start_shift",
    "drop_visible_conflict",
    "swap_verdict",
    "budget_estimated_tokens",
    "replay_different_query",
    "flip_signature_byte",
    "audit_chain_head_rewrite",
]

TAMPER_FORGERY_CLASSES = [
    "access_allowed_to_denied",
    "source_byte_start_shift",
    "drop_visible_conflict",
    "swap_verdict",
    "budget_estimated_tokens",
    "replay_different_query",
    "flip_signature_byte",
]

GATE_MARKERS = [
    "accountability-receipt-verify-check",
    "accountability-receipt-tamper-check",
    "accountability-receipt-determinism-check",
    "receipt-threat-model-check",
    "receipt-replica-invariance-check",
]

SOURCE_MARKERS = {
    "crates/cortex-receipt-verify/src/verifier.rs": VERIFIER_FUNCTION_MARKERS
    + DEFENDING_FIELDS[:10],
    "crates/cortex-receipt-verify/src/model.rs": [
        "VerifyInput",
        "ReceiptHeader",
        "ReceiptLeaves",
        "PublicKeyInput",
        "AdmittedCellInput",
    ],
    "scripts/accountability_receipt_tamper_check.py": TAMPER_FORGERY_CLASSES,
    "scripts/receipt_replica_invariance_check.py": [
        "audit_chain_head",
        "standalone verifier did not reject audit_chain_head tamper",
        "receipt-replica-invariance-check",
    ],
    "scripts/accountability_receipt_verify_check.py": [
        "cortex-receipt-verify",
        "FORBIDDEN_CRATES",
        "standalone verifier did not accept genuine fixture",
    ],
    "docs/schemas/accountability_receipt.v1.json": DEFENDING_FIELDS[:10]
    + [
        "accountability_receipt.v1",
        "blake3-256",
        "ed25519",
    ],
    "docs/spec/GCE_CONTRACT.md": [
        "docs/spec/RECEIPT_VERIFIER.md",
        "cortex-receipt-verify",
        "blake3-256",
        "ed25519",
    ],
}

MAKE_MARKERS = {
    "mk/core.mk": [
        "receipt-threat-model-check:",
        'python3 scripts/receipt_threat_model_check.py --root "." --report "$(RECEIPT_THREAT_MODEL_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "RECEIPT_THREAT_MODEL_REPORT ?= target/gce-spec/receipt-threat-model-report.json",
    ],
    "mk/phony.mk": [
        "receipt-threat-model-check",
    ],
}


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def require_markers(label: str, text: str, markers: list[str]) -> list[str]:
    return [
        f"{label}: missing marker {marker}"
        for marker in markers
        if not contains_marker(text, marker)
    ]


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    try:
        doc = read_text(root / "docs/spec/RECEIPT_VERIFIER.md")
    except FileNotFoundError:
        doc = ""
        failures.append("docs/spec/RECEIPT_VERIFIER.md: missing file")

    for label, markers in (
        ("receipt verifier doc", DOC_MARKERS),
        ("receipt verifier steps", STEP_MARKERS),
        ("receipt verifier functions", VERIFIER_FUNCTION_MARKERS),
        ("receipt verifier defending fields", DEFENDING_FIELDS),
        ("receipt verifier forgery classes", FORGERY_CLASSES),
        ("receipt verifier gates", GATE_MARKERS),
    ):
        failures.extend(require_markers(label, doc, markers))

    for relative, markers in SOURCE_MARKERS.items():
        try:
            text = read_text(root / relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        failures.extend(require_markers(relative, text, markers))

    for relative, markers in MAKE_MARKERS.items():
        try:
            text = read_text(root / relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        failures.extend(require_markers(relative, text, markers))

    return {
        "schema_version": SCHEMA_VERSION,
        "status": "failed" if failures else "passed",
        "spec": "docs/spec/RECEIPT_VERIFIER.md",
        "checked_steps": STEP_MARKERS,
        "checked_verifier_functions": VERIFIER_FUNCTION_MARKERS,
        "checked_defending_fields": DEFENDING_FIELDS,
        "checked_forgery_classes": FORGERY_CLASSES,
        "checked_gates": GATE_MARKERS,
        "checked_source_files": sorted(SOURCE_MARKERS),
        "checked_make_files": sorted(MAKE_MARKERS),
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"receipt threat model check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
