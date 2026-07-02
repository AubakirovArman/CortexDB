"""Validation checks for production-origin trust-anchor evidence."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any

from operator_evidence_validation import is_hex


def validate_expected(
    failures: list[str],
    *,
    key_attestor_key_id: str,
    key_attestor_public_key_hex: str,
    key_attestor_ref: str,
    key_attestor_public_key_ref: str,
    expected_key_attestor_key_id: str | None,
    expected_key_attestor_public_key_hex: str | None,
    expected_key_attestor_ref: str | None,
    expected_key_attestor_public_key_ref: str | None,
    publisher_key_id: str,
    publisher_public_key_hex: str,
    publisher_ref: str,
    publisher_public_key_ref: str,
    expected_publisher_key_id: str | None,
    expected_publisher_public_key_hex: str | None,
    expected_publisher_ref: str | None,
    expected_publisher_public_key_ref: str | None,
) -> None:
    expected_values = {
        "key_attestor_key_id": expected_key_attestor_key_id,
        "key_attestor_ref": expected_key_attestor_ref,
        "key_attestor_public_key_ref": expected_key_attestor_public_key_ref,
    }
    actual_values = {
        "key_attestor_key_id": key_attestor_key_id,
        "key_attestor_ref": key_attestor_ref,
        "key_attestor_public_key_ref": key_attestor_public_key_ref,
    }
    for field, expected in expected_values.items():
        if expected and actual_values[field] != expected:
            failures.append(f"{field} does not match expected key attestor trust anchor")
    if expected_key_attestor_public_key_hex:
        expected_hex = expected_key_attestor_public_key_hex
        if not is_hex(expected_hex, 64):
            failures.append("expected key attestor public key must be 64 lowercase hex characters")
        elif key_attestor_public_key_hex != expected_hex:
            failures.append(
                "key_attestor_public_key_hex does not match expected key attestor trust anchor"
            )

    expected_publisher_values = {
        "publisher_key_id": expected_publisher_key_id,
        "publisher_ref": expected_publisher_ref,
        "publisher_public_key_ref": expected_publisher_public_key_ref,
    }
    actual_publisher_values = {
        "publisher_key_id": publisher_key_id,
        "publisher_ref": publisher_ref,
        "publisher_public_key_ref": publisher_public_key_ref,
    }
    for field, expected in expected_publisher_values.items():
        if expected and actual_publisher_values[field] != expected:
            failures.append(f"{field} does not match expected trust-anchor publisher")
    if expected_publisher_public_key_hex:
        expected_hex = expected_publisher_public_key_hex
        if not is_hex(expected_hex, 64):
            failures.append("expected trust-anchor publisher public key must be 64 lowercase hex characters")
        elif publisher_public_key_hex != expected_hex:
            failures.append(
                "publisher_public_key_hex does not match expected trust-anchor publisher"
            )
    validate_independent_publisher(
        failures,
        key_attestor_key_id=key_attestor_key_id,
        key_attestor_public_key_hex=key_attestor_public_key_hex,
        key_attestor_ref=key_attestor_ref,
        key_attestor_public_key_ref=key_attestor_public_key_ref,
        publisher_key_id=publisher_key_id,
        publisher_public_key_hex=publisher_public_key_hex,
        publisher_ref=publisher_ref,
        publisher_public_key_ref=publisher_public_key_ref,
    )


def validate_independent_publisher(
    failures: list[str],
    *,
    key_attestor_key_id: str,
    key_attestor_public_key_hex: str,
    key_attestor_ref: str,
    key_attestor_public_key_ref: str,
    publisher_key_id: str,
    publisher_public_key_hex: str,
    publisher_ref: str,
    publisher_public_key_ref: str,
) -> None:
    if publisher_key_id == key_attestor_key_id:
        failures.append("publisher_key_id must be distinct from key_attestor_key_id")
    if publisher_public_key_hex == key_attestor_public_key_hex:
        failures.append("publisher_public_key_hex must be distinct from key_attestor_public_key_hex")
    if publisher_ref == key_attestor_ref:
        failures.append("publisher_ref must be distinct from key_attestor_ref")
    if publisher_public_key_ref == key_attestor_public_key_ref:
        failures.append("publisher_public_key_ref must be distinct from key_attestor_public_key_ref")


def trust_anchor_signature_fields_are_well_formed(evidence: dict[str, Any]) -> bool:
    return (
        evidence.get("signature_algorithm") == "ed25519"
        and isinstance(evidence.get("publisher_public_key_hex"), str)
        and is_hex(evidence["publisher_public_key_hex"], 64)
        and isinstance(evidence.get("signature_hex"), str)
        and is_hex(evidence["signature_hex"], 128)
    )


def trust_anchor_signing_bytes(evidence: dict[str, Any], *, signing_domain: str) -> bytes:
    subject = dict(evidence)
    subject.pop("signature_hex", None)
    subject.pop("signature_sha256_hex", None)
    payload = json.dumps(
        subject,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return signing_domain.encode("utf-8") + b"\0" + payload


def verify_trust_anchor_signature(
    evidence: dict[str, Any],
    failures: list[str],
    *,
    signing_domain: str,
) -> None:
    command = [
        "cargo",
        "run",
        "--quiet",
        "-p",
        "cortex-crypto",
        "--bin",
        "production_origin_signature",
        "--",
        "verify",
        "--public-key-hex",
        evidence["publisher_public_key_hex"],
        "--signature-hex",
        evidence["signature_hex"],
    ]
    result = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parents[1],
        input=trust_anchor_signing_bytes(evidence, signing_domain=signing_domain),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        failures.append("signature_hex must verify trust-anchor publication with publisher_public_key_hex")
