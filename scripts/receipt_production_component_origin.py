"""Shared production-origin checks for component evidence gates."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from evidence_origin import is_operator_origin_validation
from receipt_production_origin_trust_anchor_evidence import validate_trust_anchor_evidence


PRODUCTION_ORIGIN_INPUTS = {
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


def skipped_trust_anchor() -> dict[str, Any]:
    return {
        "provided": False,
        "valid": False,
        "summary": {},
        "failures": [],
        "evidence_origin": "missing",
        "synthetic_evidence": False,
        "synthetic_evidence_reasons": [],
    }


def missing_production_origin_inputs(args: Any) -> list[str]:
    return [
        env_name
        for attr, env_name in PRODUCTION_ORIGIN_INPUTS.items()
        if not getattr(args, attr)
    ]


def validate_component_trust_anchor(args: Any) -> dict[str, Any]:
    if not getattr(args, "trust_anchor_evidence", None):
        return skipped_trust_anchor()
    return validate_trust_anchor_evidence(
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


def trust_anchor_ready(value: dict[str, Any]) -> bool:
    return is_operator_origin_validation(value)


def add_production_origin_args(parser: Any) -> None:
    parser.add_argument("--trust-anchor-evidence")
    parser.add_argument("--expected-key-attestor-key-id")
    parser.add_argument("--expected-key-attestor-public-key-hex")
    parser.add_argument("--expected-key-attestor-ref")
    parser.add_argument("--expected-key-attestor-public-key-ref")
    parser.add_argument("--expected-trust-anchor-publisher-key-id")
    parser.add_argument("--expected-trust-anchor-publisher-public-key-hex")
    parser.add_argument("--expected-trust-anchor-publisher-ref")
    parser.add_argument("--expected-trust-anchor-publisher-public-key-ref")
