#!/usr/bin/env python3
"""Validate the receipt KMS/HSM custody boundary and current blocker state."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from evidence_origin import is_operator_origin_validation
from receipt_kms_hsm_evidence import validate_operator_custody_evidence
from receipt_production_component_origin import (
    add_production_origin_args,
    missing_production_origin_inputs,
    trust_anchor_ready,
    validate_component_trust_anchor,
)


REQUIRED_MARKERS = {
    "crates/cortex-server/src/main.rs": [
        "CORTEXDB_RECEIPT_SIGNING_KEY_FILE",
        "CORTEXDB_RECEIPT_SIGNING_KEY_HEX",
        "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND",
        "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX",
        "signing_seed_hex",
        "parse_receipt_signing_key_file_json",
        "receipt_external_signer_from_env",
    ],
    "crates/cortex-server/src/receipt.rs": [
        "signer: ReceiptSigner",
        "External(ReceiptExternalSigner)",
        "signed_receipt_value_with_signer",
        "set only one receipt signer mode",
    ],
    "crates/cortex-server/src/receipt_signer.rs": [
        "pub struct ReceiptExternalSigner",
        "EXTERNAL_SIGNER_REQUEST_SCHEMA",
        "Command::new(&self.command)",
        "receipt external signer signature verification failed",
    ],
    "crates/cortex-engine/src/accountability/receipt_header.rs": [
        "pub trait AccountabilityReceiptHeaderSigner",
        "sign_accountability_receipt_header_with_signer",
        "verify(&unsigned_bytes",
    ],
    "crates/cortex-engine/src/context/receipt_evidence.rs": [
        "signed_receipt_value_with_signer",
    ],
    "crates/cortex-crypto/src/receipt_key.rs": [
        'RECEIPT_SIGNING_DOMAIN: &str = "cortexdb.accountability_receipt.sign.v1"',
        "ed25519_sign",
        "ReceiptPublicKey",
    ],
    "docs/spec/ACCOUNTABILITY_RECEIPT_V1.md": [
        "External signer/KMS-HSM custody contract",
        "cortexdb.receipt_kms_hsm_custody_evidence.v1",
        "load `signing_seed_hex`",
        "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND",
        "cortexdb.accountability_receipt.sign.v1",
        "fail closed; no fallback",
    ],
    "docs/SECURITY_MODEL.md": [
        "receipt-kms-hsm-custody-check",
        "`kms_hsm_custody=false`",
        "`external_signer_runtime_supported=true`",
        "RECEIPT_KMS_HSM_CUSTODY_EVIDENCE",
    ],
    "mk/core-security-ops.mk": [
        "receipt-kms-hsm-custody-check:",
        'python3 scripts/receipt_kms_hsm_custody_check.py --root "." --report "$(RECEIPT_KMS_HSM_CUSTODY_REPORT)"',
        "$(RECEIPT_KMS_HSM_CUSTODY_ARGS)",
    ],
    "mk/vars-core.mk": [
        "RECEIPT_KMS_HSM_CUSTODY_REPORT ?= target/receipt-kms-hsm-custody/report.json",
        "RECEIPT_KMS_HSM_CUSTODY_EVIDENCE ?=",
        "RECEIPT_KMS_HSM_EXPECTED_KEY_ID ?=",
        "RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX ?=",
        "RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF ?=",
    ],
}


EXTERNAL_SIGNER_RUNTIME_MARKERS = [
    "CORTEXDB_RECEIPT_EXTERNAL_SIGNER",
    "ReceiptExternalSigner",
    "External(ReceiptExternalSigner)",
    "signed_receipt_value_with_signer",
    "AccountabilityReceiptHeaderSigner",
]

def read_text(root: Path, rel: str) -> str:
    try:
        return (root / rel).read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {rel}: {error}") from error


def missing_markers(root: Path) -> tuple[dict[str, list[str]], list[str]]:
    checked: dict[str, list[str]] = {}
    failures: list[str] = []
    for rel, markers in REQUIRED_MARKERS.items():
        text = read_text(root, rel)
        missing = [marker for marker in markers if marker not in text]
        checked[rel] = markers
        failures.extend(f"{rel}: missing marker {marker!r}" for marker in missing)
    return checked, failures


def external_runtime_supported(root: Path) -> bool:
    source_files = [
        "crates/cortex-server/src/main.rs",
        "crates/cortex-server/src/config.rs",
        "crates/cortex-server/src/receipt.rs",
        "crates/cortex-server/src/receipt_signer.rs",
        "crates/cortex-engine/src/accountability/receipt_header.rs",
        "crates/cortex-engine/src/context/receipt_evidence.rs",
        "crates/cortex-crypto/src/receipt_key.rs",
    ]
    combined = "\n".join(read_text(root, rel) for rel in source_files)
    return all(marker in combined for marker in EXTERNAL_SIGNER_RUNTIME_MARKERS)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = Path(args.root).resolve()
    checked, failures = missing_markers(root)
    has_external_runtime = external_runtime_supported(root)
    evidence = {"provided": False, "valid": False, "summary": {}, "failures": []}
    trust_anchor = {"provided": False, "valid": False, "summary": {}, "failures": []}
    missing_origin_inputs: list[str] = []
    if args.custody_evidence:
        missing_origin_inputs = missing_production_origin_inputs(args)
        for env_name in missing_origin_inputs:
            failures.append(f"missing required input for KMS/HSM custody claim: {env_name}")
        trust_anchor = validate_component_trust_anchor(args)
        failures.extend(
            f"production origin trust anchor evidence: {failure}"
            for failure in trust_anchor["failures"]
        )
        evidence = validate_operator_custody_evidence(
            Path(args.custody_evidence),
            expected_key_id=args.expected_key_id,
            expected_public_key_hex=args.expected_public_key_hex,
            expected_signer_ref=args.expected_signer_ref,
            expected_key_attestor_key_id=args.expected_key_attestor_key_id,
            expected_key_attestor_public_key_hex=args.expected_key_attestor_public_key_hex,
            expected_key_attestor_ref=args.expected_key_attestor_ref,
            expected_key_attestor_public_key_ref=args.expected_key_attestor_public_key_ref,
            require_production_origin_proof=True,
        )
        failures.extend(f"operator evidence: {failure}" for failure in evidence["failures"])
    has_operator_evidence = (
        not missing_origin_inputs
        and trust_anchor_ready(trust_anchor)
        and is_operator_origin_validation(evidence)
        and evidence.get("production_origin_proof_required") is True
        and evidence.get("production_origin_proof_valid") is True
    )
    custody_mode = (
        evidence["summary"].get("custody_mode")
        if has_operator_evidence
        else (
            "external_signer_runtime_no_kms_hsm_evidence"
            if has_external_runtime
            else "external_signer_contract_only"
        )
    )
    kms_hsm_custody = has_external_runtime and has_operator_evidence

    blockers = []
    if not has_external_runtime:
        blockers.append(
            {
                "id": "runtime_external_receipt_signer_not_implemented",
                "status": "blocked",
                "required_gate": "server receipt emission uses external signer reference instead of local signing seed",
            }
        )
    if not has_operator_evidence:
        blocker_id = "operator_kms_hsm_custody_evidence_not_implemented"
        required_gate = "operator supplies verifiable KMS/HSM key custody evidence and public key binding"
        if missing_origin_inputs:
            blocker_id = "operator_kms_hsm_custody_evidence_missing_origin_inputs"
            required_gate = (
                "KMS/HSM custody claim requires expected key-attestor trust "
                "anchor and publisher inputs"
            )
        elif trust_anchor.get("valid") is not True and trust_anchor.get("provided") is True:
            blocker_id = "operator_kms_hsm_custody_trust_anchor_invalid"
            required_gate = (
                "KMS/HSM custody claim requires valid production-origin "
                "trust-anchor publication evidence"
            )
        elif trust_anchor.get("provided") is True and not trust_anchor_ready(trust_anchor):
            blocker_id = "operator_kms_hsm_custody_trust_anchor_not_operator_origin"
            required_gate = (
                "KMS/HSM custody claim requires operator-origin trust-anchor "
                "publication evidence"
            )
        elif evidence.get("valid") is True:
            blocker_id = "operator_kms_hsm_custody_evidence_not_operator_origin"
            required_gate = "valid KMS/HSM evidence must be operator-origin, not fixture/generated/local evidence"
        elif evidence.get("provided") is True:
            blocker_id = "operator_kms_hsm_custody_evidence_invalid"
            required_gate = (
                "valid KMS/HSM evidence must include production_origin_proof "
                "bound to expected key-attestor inputs and valid trust-anchor "
                "publication evidence"
            )
        blockers.append(
            {
                "id": blocker_id,
                "status": "blocked",
                "required_gate": required_gate,
            }
        )

    return {
        "schema_version": "cortexdb.receipt_kms_hsm_custody.report.v1",
        "status": "passed" if not failures else "failed",
        "kms_hsm_custody": kms_hsm_custody,
        "custody_mode": custody_mode,
        "production_safe": kms_hsm_custody,
        "runtime_custody_path": {
            "local_seed_file_supported": True,
            "local_seed_hex_supported": True,
            "external_signer_runtime_supported": has_external_runtime,
        },
        "production_origin_trust_anchor": trust_anchor,
        "operator_evidence": evidence,
        "contract_requirements": [
            "server must not load signing_seed_hex in KMS/HSM custody mode",
            "external signer signs canonical accountability receipt header bytes",
            "signature verifies against configured key_id and public_key_hex",
            "operator evidence binds provider key ref to signer_ref and public_key_hex",
            "fail closed; no fallback to local seed in production custody mode",
        ],
        "blockers": blockers,
        "claim_boundary": (
            "operator-attested KMS/HSM custody evidence accepted; compliance "
            "certification remains a separate production-readiness gate"
            if kms_hsm_custody
            else "KMS/HSM custody boundary only; runtime supports local seed and "
            "external command signing, but KMS/HSM custody evidence is absent"
        ),
        "checked_markers": checked,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    parser.add_argument("--custody-evidence")
    parser.add_argument("--expected-key-id")
    parser.add_argument("--expected-public-key-hex")
    parser.add_argument("--expected-signer-ref")
    add_production_origin_args(parser)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except RuntimeError as error:
        print(f"receipt KMS/HSM custody check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"receipt KMS/HSM custody check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
