#!/usr/bin/env python3
"""Fail fast when production receipt operator evidence is absent or synthetic."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from compliance_certification_evidence import validate_compliance_certification_evidence
from receipt_kms_hsm_evidence import validate_operator_custody_evidence
from receipt_production_evidence_handoff_payload import operator_handoff
from receipt_production_origin_trust_anchor_evidence import validate_trust_anchor_evidence


REQUIRED_OPERATOR_INPUTS = {
    "custody_evidence": "RECEIPT_KMS_HSM_CUSTODY_EVIDENCE",
    "expected_key_id": "RECEIPT_KMS_HSM_EXPECTED_KEY_ID",
    "expected_public_key_hex": "RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX",
    "expected_signer_ref": "RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF",
    "certification_evidence": "COMPLIANCE_CERTIFICATION_EVIDENCE",
    "expected_framework": "COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK",
}

REQUIRED_PRODUCTION_ORIGIN_INPUTS = {
    "trust_anchor_evidence": "RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE",
    "expected_key_attestor_key_id": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID",
    "expected_key_attestor_public_key_hex": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX",
    "expected_key_attestor_ref": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF",
    "expected_key_attestor_public_key_ref": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF",
    "expected_trust_anchor_publisher_key_id": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID",
    "expected_trust_anchor_publisher_public_key_hex": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX",
    "expected_trust_anchor_publisher_ref": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF",
    "expected_trust_anchor_publisher_public_key_ref": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF",
}

REQUIRED_INPUTS = {
    **REQUIRED_OPERATOR_INPUTS,
    **REQUIRED_PRODUCTION_ORIGIN_INPUTS,
}


def evidence_ready(value: dict[str, Any], *, require_production_origin_proof: bool) -> bool:
    ready = (
        value.get("valid") is True
        and value.get("evidence_origin") == "operator"
        and value.get("synthetic_evidence") is not True
    )
    if require_production_origin_proof:
        ready = ready and value.get("production_origin_proof_valid") is True
    return ready


def synthetic_reason(value: dict[str, Any]) -> str:
    reasons = value.get("synthetic_evidence_reasons")
    if isinstance(reasons, list) and reasons:
        return "; ".join(str(reason) for reason in reasons)
    return "synthetic evidence origin"


def missing_inputs(args: argparse.Namespace) -> list[str]:
    missing = []
    required_inputs = dict(REQUIRED_OPERATOR_INPUTS)
    if args.require_production_origin_proof:
        required_inputs.update(REQUIRED_PRODUCTION_ORIGIN_INPUTS)
    for attr, env_name in required_inputs.items():
        if not getattr(args, attr):
            missing.append(env_name)
    return missing


def skipped_evidence() -> dict[str, Any]:
    return {
        "provided": False,
        "valid": False,
        "evidence_origin": "missing",
        "synthetic_evidence": False,
        "synthetic_evidence_reasons": [],
        "summary": {},
        "failures": [],
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    missing = missing_inputs(args)
    failures = [f"missing required input: {name}" for name in missing]

    trust_anchor = skipped_evidence()
    if args.trust_anchor_evidence:
        trust_anchor = validate_trust_anchor_evidence(
            Path(args.trust_anchor_evidence),
            expected_key_attestor_key_id=args.expected_key_attestor_key_id,
            expected_key_attestor_public_key_hex=args.expected_key_attestor_public_key_hex,
            expected_key_attestor_ref=args.expected_key_attestor_ref,
            expected_key_attestor_public_key_ref=args.expected_key_attestor_public_key_ref,
            expected_publisher_key_id=args.expected_trust_anchor_publisher_key_id,
            expected_publisher_public_key_hex=args.expected_trust_anchor_publisher_public_key_hex,
            expected_publisher_ref=args.expected_trust_anchor_publisher_ref,
            expected_publisher_public_key_ref=args.expected_trust_anchor_publisher_public_key_ref,
        )
        failures.extend(
            f"production origin trust anchor evidence: {item}"
            for item in trust_anchor["failures"]
        )
        if trust_anchor.get("valid") is True and not evidence_ready(
            trust_anchor,
            require_production_origin_proof=False,
        ):
            failures.append(
                "production origin trust anchor evidence is not operator-origin: "
                + synthetic_reason(trust_anchor)
            )

    custody = skipped_evidence()
    if args.custody_evidence:
        custody = validate_operator_custody_evidence(
            Path(args.custody_evidence),
            expected_key_id=args.expected_key_id,
            expected_public_key_hex=args.expected_public_key_hex,
            expected_signer_ref=args.expected_signer_ref,
            expected_key_attestor_key_id=args.expected_key_attestor_key_id,
            expected_key_attestor_public_key_hex=args.expected_key_attestor_public_key_hex,
            expected_key_attestor_ref=args.expected_key_attestor_ref,
            expected_key_attestor_public_key_ref=args.expected_key_attestor_public_key_ref,
            require_production_origin_proof=args.require_production_origin_proof,
        )
        failures.extend(f"KMS/HSM custody evidence: {item}" for item in custody["failures"])
        if custody.get("valid") is True and not evidence_ready(
            custody,
            require_production_origin_proof=args.require_production_origin_proof,
        ):
            failures.append(
                "KMS/HSM custody evidence is not operator-origin: "
                + synthetic_reason(custody)
            )

    certification = skipped_evidence()
    if args.certification_evidence:
        certification = validate_compliance_certification_evidence(
            Path(args.certification_evidence),
            expected_framework=args.expected_framework,
            expected_key_attestor_key_id=args.expected_key_attestor_key_id,
            expected_key_attestor_public_key_hex=args.expected_key_attestor_public_key_hex,
            expected_key_attestor_ref=args.expected_key_attestor_ref,
            expected_key_attestor_public_key_ref=args.expected_key_attestor_public_key_ref,
            require_production_origin_proof=args.require_production_origin_proof,
        )
        failures.extend(
            f"compliance certification evidence: {item}"
            for item in certification["failures"]
        )
        if certification.get("valid") is True and not evidence_ready(
            certification,
            require_production_origin_proof=args.require_production_origin_proof,
        ):
            failures.append(
                "compliance certification evidence is not operator-origin: "
                + synthetic_reason(certification)
            )

    custody_ready = evidence_ready(
        custody,
        require_production_origin_proof=args.require_production_origin_proof,
    )
    certification_ready = evidence_ready(
        certification,
        require_production_origin_proof=args.require_production_origin_proof,
    )
    trust_anchor_ready = (
        not args.require_production_origin_proof
        or evidence_ready(trust_anchor, require_production_origin_proof=False)
    )
    ready = not failures and trust_anchor_ready and custody_ready and certification_ready
    return {
        "schema_version": "cortexdb.receipt_production_evidence_preflight.v1",
        "status": "passed" if ready else "failed",
        "production_evidence_ready": ready,
        "production_origin_proof_required": args.require_production_origin_proof,
        "readiness": {
            "production_origin_trust_anchor": trust_anchor_ready,
            "kms_hsm_operator_evidence": custody_ready,
            "compliance_operator_evidence": certification_ready,
        },
        "required_inputs": list(REQUIRED_OPERATOR_INPUTS.values())
        + (
            list(REQUIRED_PRODUCTION_ORIGIN_INPUTS.values())
            if args.require_production_origin_proof
            else []
        ),
        "production_origin_expected_inputs": list(REQUIRED_PRODUCTION_ORIGIN_INPUTS.values()),
        "missing_inputs": missing,
        "operator_handoff": operator_handoff(),
        "operator_evidence": {
            "production_origin_trust_anchor": trust_anchor,
            "receipt_kms_hsm_custody": custody,
            "compliance_certification": certification,
        },
        "claim_boundary": (
            "preflight only; passing this check validates supplied evidence files "
            "and expected runtime bindings but does not replace "
            "receipt-production-ready-check"
        ),
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    parser.add_argument("--custody-evidence")
    parser.add_argument("--expected-key-id")
    parser.add_argument("--expected-public-key-hex")
    parser.add_argument("--expected-signer-ref")
    parser.add_argument("--certification-evidence")
    parser.add_argument("--expected-framework")
    parser.add_argument("--trust-anchor-evidence")
    parser.add_argument("--expected-key-attestor-key-id")
    parser.add_argument("--expected-key-attestor-public-key-hex")
    parser.add_argument("--expected-key-attestor-ref")
    parser.add_argument("--expected-key-attestor-public-key-ref")
    parser.add_argument("--expected-trust-anchor-publisher-key-id")
    parser.add_argument("--expected-trust-anchor-publisher-public-key-hex")
    parser.add_argument("--expected-trust-anchor-publisher-ref")
    parser.add_argument("--expected-trust-anchor-publisher-public-key-ref")
    parser.add_argument(
        "--require-production-origin-proof",
        action="store_true",
        help="require external production_origin_proof metadata in both evidence files",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    report = build_report(args)
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"receipt production evidence preflight passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
