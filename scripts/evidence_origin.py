"""Classify operator evidence origin without rejecting schema-valid fixtures."""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from evidence_origin_references import is_local_reference, is_temporary_local_path
    from evidence_origin_signatures import (
        PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        is_lower_hex,
        production_origin_key_attestation_signature_fields_are_well_formed,
        production_origin_key_attestation_signing_bytes,
        production_origin_signature_fields_are_well_formed,
        production_origin_statement_signing_bytes,
        verify_production_origin_key_attestation_signature,
        verify_production_origin_signature,
    )
    from operator_evidence_validation import validate_not_expired, validate_not_future
except ModuleNotFoundError:  # Allows `from scripts.evidence_origin import ...`.
    from scripts.evidence_origin_references import (
        is_local_reference,
        is_temporary_local_path,
    )
    from scripts.evidence_origin_signatures import (
        PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        is_lower_hex,
        production_origin_key_attestation_signature_fields_are_well_formed,
        production_origin_key_attestation_signing_bytes,
        production_origin_signature_fields_are_well_formed,
        production_origin_statement_signing_bytes,
        verify_production_origin_key_attestation_signature,
        verify_production_origin_signature,
    )
    from scripts.operator_evidence_validation import validate_not_expired, validate_not_future


SYNTHETIC_VALUE_MARKERS = (
    "fixture",
    "synthetic",
    "example://",
    "fixture://",
    "evidence://fixture",
)

PRODUCTION_ORIGIN_PROOF_SCHEMA = "cortexdb.operator_evidence_origin_proof.v1"
PRODUCTION_ORIGIN_PROOF_FIELDS = {
    "schema_version",
    "external_control_plane",
    "proof_ref",
    "proof_sha256_hex",
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
    "reviewed_by",
    "issued_at",
    "expires_at",
}
PRODUCTION_ORIGIN_STATEMENT_SCHEMA = "cortexdb.operator_evidence_origin_statement.v1"
PRODUCTION_ORIGIN_STATEMENT_TYPE = "production_origin_attestation"
PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA = "cortexdb.operator_evidence_origin_key_attestation.v1"
PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE = "issuer_key_attestation"
PRODUCTION_ORIGIN_KEY_ATTESTATION_FIELDS = {
    "schema_version",
    "attestation_type",
    "proof_schema_version",
    "issuer_ref",
    "issuer_key_id",
    "issuer_public_key_ref",
    "issuer_public_key_hex",
    "statement_signing_domain",
    "issuer_key_attestation_ref",
    "key_attestor_ref",
    "key_attestor_key_id",
    "key_attestor_public_key_ref",
    "key_attestor_public_key_hex",
    "key_attestation_signature_algorithm",
    "key_attestation_signature_ref",
    "reviewed_by",
    "issued_at",
    "expires_at",
    "external_control_plane",
}
PRODUCTION_ORIGIN_STATEMENT_FIELDS = {
    "schema_version",
    "statement_type",
    "proof_schema_version",
    "evidence_schema_version",
    "evidence_sha256_hex",
    "proof_ref",
    "issuer_ref",
    "issuer_key_id",
    "issuer_public_key_ref",
    "issuer_public_key_hex",
    "signed_statement_ref",
    "signature_algorithm",
    "signature_ref",
    "reviewed_by",
    "issued_at",
    "expires_at",
    "external_control_plane",
}
PRODUCTION_ORIGIN_REVIEWER_INDEPENDENCE_FIELDS = (
    "issuer_ref",
    "issuer_key_id",
    "issuer_public_key_ref",
    "key_attestor_ref",
    "key_attestor_key_id",
    "key_attestor_public_key_ref",
)
PRODUCTION_ORIGIN_PROOF_REFERENCE_FIELDS = (
    "proof_ref",
    "issuer_ref",
    "issuer_public_key_ref",
    "issuer_key_attestation_ref",
    "key_attestor_ref",
    "key_attestor_public_key_ref",
    "key_attestation_signature_ref",
    "signed_statement_ref",
    "signature_ref",
)
PRODUCTION_ORIGIN_ISSUER_ATTESTOR_INDEPENDENCE_PAIRS = (
    ("issuer_ref", "key_attestor_ref"),
    ("issuer_key_id", "key_attestor_key_id"),
    ("issuer_public_key_ref", "key_attestor_public_key_ref"),
    ("issuer_public_key_hex", "key_attestor_public_key_hex"),
)


def classify_evidence_origin(path: Path, evidence: dict[str, Any]) -> dict[str, Any]:
    reasons = non_operator_evidence_reasons(path, evidence)
    origin = non_operator_origin(path, reasons) if reasons else "operator"
    return {
        "origin": origin,
        "synthetic": bool(reasons),
        "reasons": reasons,
    }


def is_operator_origin_validation(result: dict[str, Any]) -> bool:
    return (
        result.get("valid") is True
        and result.get("evidence_origin") == "operator"
        and result.get("synthetic_evidence") is not True
    )


def production_origin_proof_summary(evidence: dict[str, Any]) -> dict[str, Any]:
    proof = evidence.get("production_origin_proof")
    if not isinstance(proof, dict):
        return {"provided": False}
    return {
        "provided": True,
        "schema_version": proof.get("schema_version"),
        "external_control_plane": proof.get("external_control_plane"),
        "proof_ref": proof.get("proof_ref"),
        "issuer_ref": proof.get("issuer_ref"),
        "issuer_key_id": proof.get("issuer_key_id"),
        "issuer_public_key_ref": proof.get("issuer_public_key_ref"),
        "issuer_public_key_hex": proof.get("issuer_public_key_hex"),
        "issuer_key_attestation_ref": proof.get("issuer_key_attestation_ref"),
        "issuer_key_attestation_sha256_hex": proof.get("issuer_key_attestation_sha256_hex"),
        "key_attestor_ref": proof.get("key_attestor_ref"),
        "key_attestor_key_id": proof.get("key_attestor_key_id"),
        "key_attestor_public_key_ref": proof.get("key_attestor_public_key_ref"),
        "key_attestor_public_key_hex": proof.get("key_attestor_public_key_hex"),
        "signed_statement_ref": proof.get("signed_statement_ref"),
        "evidence_sha256_hex": proof.get("evidence_sha256_hex"),
        "signed_statement_sha256_hex": proof.get("signed_statement_sha256_hex"),
        "signature_algorithm": proof.get("signature_algorithm"),
        "signature_ref": proof.get("signature_ref"),
        "signature_sha256_hex": proof.get("signature_sha256_hex"),
        "signature_verified": proof.get("signature_verified"),
    }


def production_origin_proof_failures(
    evidence: dict[str, Any],
    *,
    expected_key_attestor_key_id: str | None = None,
    expected_key_attestor_public_key_hex: str | None = None,
    expected_key_attestor_ref: str | None = None,
    expected_key_attestor_public_key_ref: str | None = None,
) -> list[str]:
    proof = evidence.get("production_origin_proof")
    if not isinstance(proof, dict):
        return ["production_origin_proof must be an object"]

    failures: list[str] = []
    if proof.get("schema_version") != PRODUCTION_ORIGIN_PROOF_SCHEMA:
        failures.append(
            f"production_origin_proof.schema_version must be {PRODUCTION_ORIGIN_PROOF_SCHEMA}"
        )
    if proof.get("external_control_plane") is not True:
        failures.append("production_origin_proof.external_control_plane must be true")
    for field in sorted(set(proof) - PRODUCTION_ORIGIN_PROOF_FIELDS):
        failures.append(f"production_origin_proof.{field} is not allowed")
    for field in (
        "proof_ref",
        "proof_sha256_hex",
        "issuer_ref",
        "issuer_key_id",
        "issuer_public_key_ref",
        "issuer_public_key_hex",
        "issuer_key_attestation_ref",
        "issuer_key_attestation_sha256_hex",
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
        "evidence_sha256_hex",
        "signature_algorithm",
        "signature_ref",
        "signature_sha256_hex",
        "signature_hex",
        "reviewed_by",
        "issued_at",
        "expires_at",
    ):
        value = proof.get(field)
        if not isinstance(value, str) or not value.strip():
            failures.append(f"production_origin_proof.{field} must be a non-empty string")
        elif value != value.strip():
            failures.append(f"production_origin_proof.{field} must not include surrounding whitespace")
    for field in ("issuer_key_id", "key_attestor_key_id"):
        value = proof.get(field)
        if isinstance(value, str) and any(character.isspace() for character in value):
            failures.append(f"production_origin_proof.{field} must contain no whitespace")
    reviewed_by = proof.get("reviewed_by")
    if isinstance(reviewed_by, str) and any(character.isspace() for character in reviewed_by):
        failures.append("production_origin_proof.reviewed_by must contain no whitespace")
    for field in PRODUCTION_ORIGIN_PROOF_REFERENCE_FIELDS:
        value = proof.get(field)
        if isinstance(value, str) and any(character.isspace() for character in value):
            failures.append(f"production_origin_proof.{field} must contain no whitespace")
    for field in (
        "proof_sha256_hex",
        "signed_statement_sha256_hex",
        "evidence_sha256_hex",
        "signature_sha256_hex",
        "issuer_key_attestation_sha256_hex",
        "key_attestation_signature_sha256_hex",
    ):
        digest = proof.get(field)
        if isinstance(digest, str) and not is_lower_hex(digest, 64):
            failures.append(f"production_origin_proof.{field} must be 64 lowercase hex characters")
    issuer_public_key_hex = proof.get("issuer_public_key_hex")
    if isinstance(issuer_public_key_hex, str) and not is_lower_hex(issuer_public_key_hex, 64):
        failures.append("production_origin_proof.issuer_public_key_hex must be 64 lowercase hex characters")
    key_attestor_public_key_hex = proof.get("key_attestor_public_key_hex")
    if isinstance(key_attestor_public_key_hex, str) and not is_lower_hex(key_attestor_public_key_hex, 64):
        failures.append("production_origin_proof.key_attestor_public_key_hex must be 64 lowercase hex characters")
    validate_expected_key_attestor(
        proof,
        failures,
        expected_key_attestor_key_id=expected_key_attestor_key_id,
        expected_key_attestor_public_key_hex=expected_key_attestor_public_key_hex,
        expected_key_attestor_ref=expected_key_attestor_ref,
        expected_key_attestor_public_key_ref=expected_key_attestor_public_key_ref,
    )
    validate_issuer_attestor_independence(proof, failures)
    validate_reviewer_independence(proof, failures)
    signature_hex = proof.get("signature_hex")
    if isinstance(signature_hex, str) and not is_lower_hex(signature_hex, 128):
        failures.append("production_origin_proof.signature_hex must be 128 lowercase hex characters")
    key_attestation_signature_hex = proof.get("key_attestation_signature_hex")
    if isinstance(key_attestation_signature_hex, str) and not is_lower_hex(key_attestation_signature_hex, 128):
        failures.append("production_origin_proof.key_attestation_signature_hex must be 128 lowercase hex characters")
    if proof.get("signature_algorithm") != "ed25519":
        failures.append("production_origin_proof.signature_algorithm must be ed25519")
    if proof.get("key_attestation_signature_algorithm") != "ed25519":
        failures.append("production_origin_proof.key_attestation_signature_algorithm must be ed25519")
    signature_digest = proof.get("signature_sha256_hex")
    if isinstance(signature_hex, str) and is_lower_hex(signature_hex, 128) and isinstance(signature_digest, str):
        if signature_digest != hashlib.sha256(bytes.fromhex(signature_hex)).hexdigest():
            failures.append("production_origin_proof.signature_sha256_hex must match signature_hex bytes")
    key_attestation_signature_digest = proof.get("key_attestation_signature_sha256_hex")
    if (
        isinstance(key_attestation_signature_hex, str)
        and is_lower_hex(key_attestation_signature_hex, 128)
        and isinstance(key_attestation_signature_digest, str)
    ):
        if key_attestation_signature_digest != hashlib.sha256(bytes.fromhex(key_attestation_signature_hex)).hexdigest():
            failures.append("production_origin_proof.key_attestation_signature_sha256_hex must match key_attestation_signature_hex bytes")
    proof_digest = proof.get("proof_sha256_hex")
    if isinstance(proof_digest, str) and proof_digest != production_origin_proof_body_sha256_hex(proof):
        failures.append("production_origin_proof.proof_sha256_hex must match production_origin_proof without proof_sha256_hex")
    expected_evidence_digest = production_origin_subject_sha256_hex(evidence)
    evidence_digest = proof.get("evidence_sha256_hex")
    if isinstance(evidence_digest, str) and evidence_digest != expected_evidence_digest:
        failures.append("production_origin_proof.evidence_sha256_hex must match evidence JSON without production_origin_proof")
    validate_production_origin_key_attestation(proof, failures)
    validate_production_origin_statement(evidence, proof, expected_evidence_digest, failures)
    for field in PRODUCTION_ORIGIN_PROOF_REFERENCE_FIELDS:
        value = proof.get(field)
        if isinstance(value, str):
            lowered = value.lower()
            if any(marker in lowered for marker in SYNTHETIC_VALUE_MARKERS):
                failures.append(f"production_origin_proof.{field} contains a synthetic evidence marker")
            if is_local_reference(lowered):
                failures.append(
                    f"production_origin_proof.{field} contains a local/generated evidence reference"
                )
    issued_at = parse_iso_timestamp(proof.get("issued_at"))
    expires_at = parse_iso_timestamp(proof.get("expires_at"))
    if isinstance(proof.get("issued_at"), str) and issued_at is None:
        failures.append("production_origin_proof.issued_at must be timezone-aware ISO-8601")
    if isinstance(proof.get("expires_at"), str) and expires_at is None:
        failures.append("production_origin_proof.expires_at must be timezone-aware ISO-8601")
    if issued_at and expires_at and expires_at <= issued_at:
        failures.append("production_origin_proof.expires_at must be after issued_at")
    validate_not_future(issued_at, failures, label="production_origin_proof.issued_at")
    validate_not_expired(expires_at, failures, label="production_origin_proof.expires_at")
    return failures


def validate_expected_key_attestor(
    proof: dict[str, Any],
    failures: list[str],
    *,
    expected_key_attestor_key_id: str | None,
    expected_key_attestor_public_key_hex: str | None,
    expected_key_attestor_ref: str | None,
    expected_key_attestor_public_key_ref: str | None,
) -> None:
    expected_values = {
        "key_attestor_key_id": expected_key_attestor_key_id,
        "key_attestor_public_key_ref": expected_key_attestor_public_key_ref,
        "key_attestor_ref": expected_key_attestor_ref,
    }
    for field, expected in expected_values.items():
        if expected and proof.get(field) != expected:
            failures.append(f"production_origin_proof.{field} does not match expected key attestor trust anchor")
    if expected_key_attestor_public_key_hex:
        expected_hex = expected_key_attestor_public_key_hex
        if not is_lower_hex(expected_hex, 64):
            failures.append("expected key attestor public key must be 64 lowercase hex characters")
        elif proof.get("key_attestor_public_key_hex") != expected_hex:
            failures.append(
                "production_origin_proof.key_attestor_public_key_hex does not match expected key attestor trust anchor"
            )


def validate_issuer_attestor_independence(
    proof: dict[str, Any],
    failures: list[str],
) -> None:
    for issuer_field, attestor_field in PRODUCTION_ORIGIN_ISSUER_ATTESTOR_INDEPENDENCE_PAIRS:
        issuer_value = proof.get(issuer_field)
        attestor_value = proof.get(attestor_field)
        if not isinstance(issuer_value, str) or not isinstance(attestor_value, str):
            continue
        if issuer_value.strip() and issuer_value.strip() == attestor_value.strip():
            failures.append(
                f"production_origin_proof.{issuer_field} must be distinct from {attestor_field}"
            )


def validate_reviewer_independence(proof: dict[str, Any], failures: list[str]) -> None:
    reviewed_by = proof.get("reviewed_by")
    if not isinstance(reviewed_by, str):
        return
    reviewed_identity = reviewed_by.strip()
    if not reviewed_identity:
        return
    for field in PRODUCTION_ORIGIN_REVIEWER_INDEPENDENCE_FIELDS:
        value = proof.get(field)
        if isinstance(value, str) and reviewed_identity == value.strip():
            failures.append(
                f"production_origin_proof.reviewed_by must be distinct from {field}"
            )


def validate_production_origin_key_attestation(
    proof: dict[str, Any],
    failures: list[str],
) -> None:
    attestation = proof.get("issuer_key_attestation")
    if not isinstance(attestation, dict):
        failures.append("production_origin_proof.issuer_key_attestation must be an object")
        return
    attestation_digest = proof.get("issuer_key_attestation_sha256_hex")
    if isinstance(attestation_digest, str) and attestation_digest != canonical_json_sha256_hex(attestation):
        failures.append(
            "production_origin_proof.issuer_key_attestation_sha256_hex must match issuer_key_attestation"
        )
    for field in sorted(set(attestation) - PRODUCTION_ORIGIN_KEY_ATTESTATION_FIELDS):
        failures.append(f"production_origin_proof.issuer_key_attestation.{field} is not allowed")
    for field in (
        "issuer_key_attestation_sha256_hex",
        "key_attestation_signature_hex",
        "key_attestation_signature_sha256_hex",
    ):
        if field in attestation:
            failures.append(f"production_origin_proof.issuer_key_attestation.{field} must not be present")
    expected_values = {
        "schema_version": PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
        "attestation_type": PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
        "proof_schema_version": PRODUCTION_ORIGIN_PROOF_SCHEMA,
        "issuer_ref": proof.get("issuer_ref"),
        "issuer_key_id": proof.get("issuer_key_id"),
        "issuer_public_key_ref": proof.get("issuer_public_key_ref"),
        "issuer_public_key_hex": proof.get("issuer_public_key_hex"),
        "statement_signing_domain": PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        "issuer_key_attestation_ref": proof.get("issuer_key_attestation_ref"),
        "key_attestor_ref": proof.get("key_attestor_ref"),
        "key_attestor_key_id": proof.get("key_attestor_key_id"),
        "key_attestor_public_key_ref": proof.get("key_attestor_public_key_ref"),
        "key_attestor_public_key_hex": proof.get("key_attestor_public_key_hex"),
        "key_attestation_signature_algorithm": proof.get("key_attestation_signature_algorithm"),
        "key_attestation_signature_ref": proof.get("key_attestation_signature_ref"),
        "reviewed_by": proof.get("reviewed_by"),
        "issued_at": proof.get("issued_at"),
        "expires_at": proof.get("expires_at"),
        "external_control_plane": True,
    }
    for field, expected in expected_values.items():
        if attestation.get(field) != expected:
            failures.append(f"production_origin_proof.issuer_key_attestation.{field} must match proof")
    if production_origin_key_attestation_signature_fields_are_well_formed(proof):
        verify_production_origin_key_attestation_signature(proof, attestation, failures)


def validate_production_origin_statement(
    evidence: dict[str, Any],
    proof: dict[str, Any],
    expected_evidence_digest: str,
    failures: list[str],
) -> None:
    statement = proof.get("signed_statement")
    if not isinstance(statement, dict):
        failures.append("production_origin_proof.signed_statement must be an object")
        return
    statement_digest = proof.get("signed_statement_sha256_hex")
    if isinstance(statement_digest, str) and statement_digest != canonical_json_sha256_hex(statement):
        failures.append(
            "production_origin_proof.signed_statement_sha256_hex must match signed_statement"
        )
    for field in sorted(set(statement) - PRODUCTION_ORIGIN_STATEMENT_FIELDS):
        failures.append(f"production_origin_proof.signed_statement.{field} is not allowed")
    for field in ("signed_statement_sha256_hex", "signature_hex", "signature_sha256_hex"):
        if field in statement:
            failures.append(f"production_origin_proof.signed_statement.{field} must not be present")
    expected_values = {
        "schema_version": PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
        "statement_type": PRODUCTION_ORIGIN_STATEMENT_TYPE,
        "proof_schema_version": PRODUCTION_ORIGIN_PROOF_SCHEMA,
        "evidence_schema_version": evidence.get("schema_version"),
        "evidence_sha256_hex": expected_evidence_digest,
        "proof_ref": proof.get("proof_ref"),
        "issuer_ref": proof.get("issuer_ref"),
        "issuer_key_id": proof.get("issuer_key_id"),
        "issuer_public_key_ref": proof.get("issuer_public_key_ref"),
        "issuer_public_key_hex": proof.get("issuer_public_key_hex"),
        "signed_statement_ref": proof.get("signed_statement_ref"),
        "signature_algorithm": proof.get("signature_algorithm"),
        "signature_ref": proof.get("signature_ref"),
        "reviewed_by": proof.get("reviewed_by"),
        "issued_at": proof.get("issued_at"),
        "expires_at": proof.get("expires_at"),
        "external_control_plane": True,
    }
    for field, expected in expected_values.items():
        if statement.get(field) != expected:
            failures.append(f"production_origin_proof.signed_statement.{field} must match proof/evidence")
    if production_origin_signature_fields_are_well_formed(proof):
        verify_production_origin_signature(proof, statement, failures)


def production_origin_subject_sha256_hex(evidence: dict[str, Any]) -> str:
    subject = dict(evidence)
    subject.pop("production_origin_proof", None)
    return canonical_json_sha256_hex(subject)


def production_origin_proof_body_sha256_hex(proof: dict[str, Any]) -> str:
    subject = dict(proof)
    subject.pop("proof_sha256_hex", None)
    return canonical_json_sha256_hex(subject)


def canonical_json_sha256_hex(value: Any) -> str:
    payload = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def parse_iso_timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        return None
    return parsed.astimezone(timezone.utc)


def non_operator_origin(path: Path, reasons: list[str]) -> str:
    normalized_paths = normalized_evidence_paths(path)
    if any(is_generated_local_path(item) for item in normalized_paths):
        return "generated_local_artifact"
    if any(is_temporary_local_path(item) for item in normalized_paths):
        return "temporary_local_artifact"
    if any("local/generated evidence reference" in reason for reason in reasons):
        return "local_reference_artifact"
    return "synthetic_fixture"


def non_operator_evidence_reasons(path: Path, evidence: dict[str, Any]) -> list[str]:
    reasons: list[str] = []
    for normalized_path in normalized_evidence_paths(path):
        if is_fixture_path(normalized_path):
            reasons.append("evidence path is under fixtures/")
        if is_generated_local_path(normalized_path):
            reasons.append("evidence path is under target/")
        if is_temporary_local_path(normalized_path):
            reasons.append("evidence path is under a temporary local directory")
    for field_path, value in string_values(evidence):
        lowered = value.lower()
        if any(marker in lowered for marker in SYNTHETIC_VALUE_MARKERS):
            reasons.append(f"{field_path} contains a synthetic evidence marker")
        if is_local_reference(lowered):
            reasons.append(f"{field_path} contains a local/generated evidence reference")
    return sorted(set(reasons))


def normalized_evidence_paths(path: Path) -> list[str]:
    paths = [path.as_posix().lower()]
    try:
        resolved = path.resolve(strict=True).as_posix().lower()
    except OSError:
        return paths
    if resolved not in paths:
        paths.append(resolved)
    return paths


def is_fixture_path(normalized_path: str) -> bool:
    return normalized_path.startswith("fixtures/") or "/fixtures/" in normalized_path


def is_generated_local_path(normalized_path: str) -> bool:
    return normalized_path.startswith("target/") or "/target/" in normalized_path


def string_values(value: Any, prefix: str = "$") -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    if isinstance(value, dict):
        for key, child in value.items():
            found.extend(string_values(child, f"{prefix}.{key}"))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            found.extend(string_values(child, f"{prefix}[{index}]"))
    elif isinstance(value, str):
        found.append((prefix, value))
    return found
