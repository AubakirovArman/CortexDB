"""Signature and key-attestation verification for production-origin proofs."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any


PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN = "cortexdb.operator_evidence_origin_statement.sign.v1"
PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN = "cortexdb.operator_evidence_origin_key_attestation.sign.v1"


def production_origin_statement_signing_bytes(statement: dict[str, Any]) -> bytes:
    payload = json.dumps(
        statement,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN.encode("utf-8") + b"\0" + payload


def production_origin_signature_fields_are_well_formed(proof: dict[str, Any]) -> bool:
    return (
        proof.get("signature_algorithm") == "ed25519"
        and isinstance(proof.get("issuer_public_key_hex"), str)
        and is_lower_hex(proof["issuer_public_key_hex"], 64)
        and isinstance(proof.get("signature_hex"), str)
        and is_lower_hex(proof["signature_hex"], 128)
    )


def production_origin_key_attestation_signature_fields_are_well_formed(proof: dict[str, Any]) -> bool:
    return (
        proof.get("key_attestation_signature_algorithm") == "ed25519"
        and isinstance(proof.get("key_attestor_public_key_hex"), str)
        and is_lower_hex(proof["key_attestor_public_key_hex"], 64)
        and isinstance(proof.get("key_attestation_signature_hex"), str)
        and is_lower_hex(proof["key_attestation_signature_hex"], 128)
    )


def verify_production_origin_signature(
    proof: dict[str, Any],
    statement: dict[str, Any],
    failures: list[str],
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
        proof["issuer_public_key_hex"],
        "--signature-hex",
        proof["signature_hex"],
    ]
    result = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parents[1],
        input=production_origin_statement_signing_bytes(statement),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        failures.append(
            "production_origin_proof.signature_hex must verify signed_statement with issuer_public_key_hex"
        )


def production_origin_key_attestation_signing_bytes(attestation: dict[str, Any]) -> bytes:
    payload = json.dumps(
        attestation,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN.encode("utf-8") + b"\0" + payload


def verify_production_origin_key_attestation_signature(
    proof: dict[str, Any],
    attestation: dict[str, Any],
    failures: list[str],
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
        proof["key_attestor_public_key_hex"],
        "--signature-hex",
        proof["key_attestation_signature_hex"],
    ]
    result = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parents[1],
        input=production_origin_key_attestation_signing_bytes(attestation),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        failures.append(
            "production_origin_proof.key_attestation_signature_hex must verify issuer_key_attestation with key_attestor_public_key_hex"
        )


def is_lower_hex(value: str, length: int) -> bool:
    return len(value) == length and all(character in "0123456789abcdef" for character in value)
