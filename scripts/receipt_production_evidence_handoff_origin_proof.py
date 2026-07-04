#!/usr/bin/env python3
"""Production-origin proof/statement/key-attestation checklist checks for the handoff."""

from __future__ import annotations

from typing import Any

try:
    from evidence_origin import (
        PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
        PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
        PRODUCTION_ORIGIN_PROOF_SCHEMA,
        PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
        PRODUCTION_ORIGIN_STATEMENT_TYPE,
    )
except ModuleNotFoundError:  # Allows `from scripts.receipt_production_evidence_handoff_origin_proof import ...`.
    from scripts.evidence_origin import (
        PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
        PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
        PRODUCTION_ORIGIN_PROOF_SCHEMA,
        PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
        PRODUCTION_ORIGIN_STATEMENT_TYPE,
    )


def require(condition: bool, failures: list[str], message: str) -> None:
    if not condition:
        failures.append(message)


def check_production_origin_proof(component: dict[str, Any], label: str, failures: list[str]) -> None:
    fields = component.get("production_ready_required_top_level_fields")
    require(fields == ["production_origin_proof"], failures, f"{label} production proof field drifted")
    proof = component.get("production_origin_proof_requirements")
    if not isinstance(proof, dict):
        failures.append(f"{label} production origin proof requirements must be an object")
        return
    require(
        proof.get("schema_version") == PRODUCTION_ORIGIN_PROOF_SCHEMA,
        failures,
        f"{label} production origin proof schema drifted",
    )
    require(
        "proof_ref" in proof.get("required_fields", []),
        failures,
        f"{label} production origin proof ref missing",
    )
    for field in (
        "issuer_ref",
        "issuer_key_id",
        "issuer_public_key_ref",
        "issuer_public_key_hex",
        "issuer_key_attestation_ref",
        "issuer_key_attestation_sha256_hex",
        "issuer_key_attestation",
        "key_attestor_ref",
        "key_attestor_key_id",
        "key_attestor_public_key_ref",
        "key_attestor_public_key_hex",
        "key_attestation_signature_algorithm",
        "key_attestation_signature_ref",
        "key_attestation_signature_sha256_hex",
        "key_attestation_signature_hex",
        "signed_statement_ref",
        "signed_statement_sha256_hex",
        "signed_statement",
        "evidence_sha256_hex",
        "signature_algorithm",
        "signature_ref",
        "signature_sha256_hex",
        "signature_hex",
        "expires_at",
    ):
        require(
            field in proof.get("required_fields", []),
            failures,
            f"{label} production origin proof {field} missing",
        )
    require(
        proof.get("required_values", {}).get("external_control_plane") is True,
        failures,
        f"{label} production origin proof external control plane drifted",
    )
    require(
        proof.get("required_values", {}).get("evidence_sha256_hex"),
        failures,
        f"{label} production origin proof evidence digest guidance missing",
    )
    require(
        proof.get("required_values", {}).get("proof_sha256_hex"),
        failures,
        f"{label} production origin proof self digest guidance missing",
    )
    require(
        proof.get("required_values", {}).get("signed_statement_sha256_hex"),
        failures,
        f"{label} production origin proof statement digest guidance missing",
    )
    require(
        "surrounding whitespace" in str(proof.get("required_values", {}).get("string_values", "")),
        failures,
        f"{label} production origin proof string canonicalization guidance missing",
    )
    require(
        "no whitespace" in str(proof.get("required_values", {}).get("key_ids", "")),
        failures,
        f"{label} production origin proof key-id canonicalization guidance missing",
    )
    require(
        "no whitespace" in str(proof.get("required_values", {}).get("reviewer_identity", "")),
        failures,
        f"{label} production origin proof reviewer canonicalization guidance missing",
    )
    for key in ("issued_at", "expires_at"):
        require(
            "timezone-aware" in str(proof.get("required_values", {}).get(key, "")),
            failures,
            f"{label} production origin proof {key} timezone guidance missing",
        )
    require(
        proof.get("required_values", {}).get("signature_algorithm") == "ed25519",
        failures,
        f"{label} production origin proof signature algorithm drifted",
    )
    require(
        proof.get("required_values", {}).get("signature_hex"),
        failures,
        f"{label} production origin proof signature hex guidance missing",
    )
    require(
        proof.get("required_values", {}).get("key_attestation_signature_algorithm") == "ed25519",
        failures,
        f"{label} production origin proof key attestation signature algorithm drifted",
    )
    require(
        proof.get("required_values", {}).get("key_attestor_trust_anchor_binding"),
        failures,
        f"{label} production origin proof key attestor trust-anchor guidance missing",
    )
    require(
        proof.get("required_values", {}).get("issuer_attestor_independence"),
        failures,
        f"{label} production origin proof issuer-attestor independence guidance missing",
    )
    require(
        proof.get("required_values", {}).get("reviewer_independence"),
        failures,
        f"{label} production origin proof reviewer independence guidance missing",
    )
    require(
        proof.get("required_values", {}).get("key_attestation_signature_hex"),
        failures,
        f"{label} production origin proof key attestation signature guidance missing",
    )
    reference_boundary = str(proof.get("reference_boundary", ""))
    require(
        "non-local" in reference_boundary and "whitespace" in reference_boundary,
        failures,
        f"{label} production origin proof reference-boundary guidance missing",
    )
    key_attestation = proof.get("issuer_key_attestation_requirements")
    if not isinstance(key_attestation, dict):
        failures.append(f"{label} production origin key attestation requirements must be an object")
        return
    require(
        key_attestation.get("schema_version") == PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
        failures,
        f"{label} production origin key attestation schema drifted",
    )
    require(
        key_attestation.get("attestation_type") == PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
        failures,
        f"{label} production origin key attestation type drifted",
    )
    for field in (
        "issuer_public_key_hex",
        "statement_signing_domain",
        "key_attestor_ref",
        "key_attestor_key_id",
        "key_attestor_public_key_ref",
        "key_attestor_public_key_hex",
        "key_attestation_signature_algorithm",
        "key_attestation_signature_ref",
    ):
        require(
            field in key_attestation.get("required_fields", []),
            failures,
            f"{label} production origin key attestation {field} missing",
        )
    require(
        "key_attestation_signature_sha256_hex" not in key_attestation.get("required_fields", []),
        failures,
        f"{label} production origin key attestation signature digest must stay outside signed bytes",
    )
    require(
        key_attestation.get("signing_domain") == PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        failures,
        f"{label} production origin key attestation signing domain drifted",
    )
    statement = proof.get("signed_statement_requirements")
    if not isinstance(statement, dict):
        failures.append(f"{label} production origin statement requirements must be an object")
        return
    require(
        statement.get("schema_version") == PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
        failures,
        f"{label} production origin statement schema drifted",
    )
    require(
        statement.get("statement_type") == PRODUCTION_ORIGIN_STATEMENT_TYPE,
        failures,
        f"{label} production origin statement type drifted",
    )
    for field in (
        "evidence_schema_version",
        "evidence_sha256_hex",
        "issuer_public_key_ref",
        "issuer_public_key_hex",
        "signature_ref",
    ):
        require(
            field in statement.get("required_fields", []),
            failures,
            f"{label} production origin statement {field} missing",
        )
    require(
        "signature_sha256_hex" not in statement.get("required_fields", []),
        failures,
        f"{label} production origin statement signature digest must stay outside signed bytes",
    )
    require(
        statement.get("signing_domain") == PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        failures,
        f"{label} production origin statement signing domain drifted",
    )
