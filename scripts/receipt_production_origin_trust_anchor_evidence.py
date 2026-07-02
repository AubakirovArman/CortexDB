#!/usr/bin/env python3
"""Validate operator-published production-origin key-attestor trust anchors."""

from __future__ import annotations

import json
import hashlib
from pathlib import Path
from typing import Any

from evidence_origin import classify_evidence_origin
from operator_evidence_validation import (
    FORBIDDEN_SECRET_KEYS,
    forbidden_secret_paths,
    invalid_result,
    is_hex,
    parse_timestamp,
    string_field,
    validate_evidence_artifacts,
    validate_not_future,
    validate_not_expired,
    validate_non_local_reference,
    validate_required_controls,
)
from receipt_production_origin_trust_anchor_checks import (
    trust_anchor_signature_fields_are_well_formed,
    validate_expected,
    verify_trust_anchor_signature,
)


EVIDENCE_SCHEMA = "cortexdb.operator_evidence_origin_trust_anchor.v1"
TRUST_ANCHOR_TYPE = "key_attestor_publication"
SIGNING_DOMAIN = "cortexdb.operator_evidence_origin_trust_anchor.sign.v1"
TRUST_ANCHOR_EVIDENCE_FIELDS = {
    "schema_version",
    "trust_anchor_type",
    "external_control_plane",
    "key_attestor_key_id",
    "key_attestor_public_key_hex",
    "key_attestor_ref",
    "key_attestor_public_key_ref",
    "publisher",
    "publisher_ref",
    "publisher_key_id",
    "publisher_public_key_ref",
    "publisher_public_key_hex",
    "publication_ref",
    "signature_algorithm",
    "signature_ref",
    "signature_hex",
    "signature_sha256_hex",
    "published_at",
    "valid_until",
    "controls",
    "evidence_artifacts",
}
REQUIRED_CONTROLS = {
    "attestor_key_identity_reviewed",
    "attestor_public_key_published",
    "publication_digest_recorded",
    "production_origin_scope_reviewed",
}
EVIDENCE_ARTIFACT_KINDS = {"publication", "publisher-key"}
NON_LOCAL_REFERENCE_FIELDS = {
    "key_attestor_ref",
    "key_attestor_public_key_ref",
    "publication_ref",
    "publisher_ref",
    "publisher_public_key_ref",
    "signature_ref",
}
def validate_trust_anchor_evidence(
    path: Path,
    *,
    expected_key_attestor_key_id: str | None,
    expected_key_attestor_public_key_hex: str | None,
    expected_key_attestor_ref: str | None,
    expected_key_attestor_public_key_ref: str | None,
    expected_publisher_key_id: str | None,
    expected_publisher_public_key_hex: str | None,
    expected_publisher_ref: str | None,
    expected_publisher_public_key_ref: str | None,
) -> dict[str, Any]:
    failures: list[str] = []
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        return invalid_result(path, [f"failed to read evidence: {error}"])
    except json.JSONDecodeError as error:
        return invalid_result(path, [f"failed to parse evidence JSON: {error}"])
    if not isinstance(evidence, dict):
        return invalid_result(path, ["evidence must be a JSON object"])

    origin = classify_evidence_origin(path, evidence)
    for field_path in forbidden_secret_paths(evidence):
        failures.append(f"forbidden secret material field present: {field_path}")
    for field in sorted(set(evidence) - TRUST_ANCHOR_EVIDENCE_FIELDS):
        failures.append(f"production_origin_trust_anchor_evidence.{field} is not allowed")

    schema_version = string_field(evidence, "schema_version", failures)
    if schema_version != EVIDENCE_SCHEMA:
        failures.append(f"schema_version must be {EVIDENCE_SCHEMA}")
    trust_anchor_type = string_field(evidence, "trust_anchor_type", failures)
    if trust_anchor_type != TRUST_ANCHOR_TYPE:
        failures.append(f"trust_anchor_type must be {TRUST_ANCHOR_TYPE}")
    key_attestor_key_id = string_field(evidence, "key_attestor_key_id", failures)
    key_attestor_public_key_hex = string_field(
        evidence,
        "key_attestor_public_key_hex",
        failures,
    )
    key_attestor_ref = string_field(evidence, "key_attestor_ref", failures)
    key_attestor_public_key_ref = string_field(
        evidence,
        "key_attestor_public_key_ref",
        failures,
    )
    publisher = string_field(evidence, "publisher", failures)
    publisher_ref = string_field(evidence, "publisher_ref", failures)
    publisher_key_id = string_field(evidence, "publisher_key_id", failures)
    publisher_public_key_ref = string_field(evidence, "publisher_public_key_ref", failures)
    publisher_public_key_hex = string_field(
        evidence,
        "publisher_public_key_hex",
        failures,
    )
    publication_ref = string_field(evidence, "publication_ref", failures)
    signature_algorithm = string_field(evidence, "signature_algorithm", failures)
    signature_ref = string_field(evidence, "signature_ref", failures)
    signature_hex = string_field(evidence, "signature_hex", failures)
    signature_sha256_hex = string_field(evidence, "signature_sha256_hex", failures)
    published_at = string_field(evidence, "published_at", failures)
    valid_until = string_field(evidence, "valid_until", failures)

    for field in sorted(NON_LOCAL_REFERENCE_FIELDS):
        validate_external_reference(
            evidence.get(field),
            failures,
            field_name=field,
        )
    if not is_hex(key_attestor_public_key_hex, 64):
        failures.append("key_attestor_public_key_hex must be 64 lowercase hex characters")
    if not is_hex(publisher_public_key_hex, 64):
        failures.append("publisher_public_key_hex must be 64 lowercase hex characters")
    if signature_algorithm != "ed25519":
        failures.append("signature_algorithm must be ed25519")
    if not is_hex(signature_hex, 128):
        failures.append("signature_hex must be 128 lowercase hex characters")
    if not is_hex(signature_sha256_hex, 64):
        failures.append("signature_sha256_hex must be 64 lowercase hex characters")
    if is_hex(signature_hex, 128) and is_hex(signature_sha256_hex, 64):
        if signature_sha256_hex != hashlib.sha256(bytes.fromhex(signature_hex)).hexdigest():
            failures.append("signature_sha256_hex must match signature_hex bytes")
    if evidence.get("external_control_plane") is not True:
        failures.append("external_control_plane must be true")
    for field_name, value in (
        ("key_attestor_key_id", key_attestor_key_id),
        ("publisher_key_id", publisher_key_id),
    ):
        if any(character.isspace() for character in value):
            failures.append(f"{field_name} must contain no whitespace")
    published_at_time = parse_timestamp(published_at, "published_at", failures)
    valid_until_time = parse_timestamp(valid_until, "valid_until", failures)
    validate_not_future(published_at_time, failures, label="published_at")
    if published_at_time and valid_until_time:
        if valid_until_time <= published_at_time:
            failures.append("valid_until must be after published_at")
        validate_not_expired(valid_until_time, failures, label="valid_until")
    validate_expected(
        failures,
        key_attestor_key_id=key_attestor_key_id,
        key_attestor_public_key_hex=key_attestor_public_key_hex,
        key_attestor_ref=key_attestor_ref,
        key_attestor_public_key_ref=key_attestor_public_key_ref,
        expected_key_attestor_key_id=expected_key_attestor_key_id,
        expected_key_attestor_public_key_hex=expected_key_attestor_public_key_hex,
        expected_key_attestor_ref=expected_key_attestor_ref,
        expected_key_attestor_public_key_ref=expected_key_attestor_public_key_ref,
        publisher_key_id=publisher_key_id,
        publisher_public_key_hex=publisher_public_key_hex,
        publisher_ref=publisher_ref,
        publisher_public_key_ref=publisher_public_key_ref,
        expected_publisher_key_id=expected_publisher_key_id,
        expected_publisher_public_key_hex=expected_publisher_public_key_hex,
        expected_publisher_ref=expected_publisher_ref,
        expected_publisher_public_key_ref=expected_publisher_public_key_ref,
    )
    if trust_anchor_signature_fields_are_well_formed(evidence):
        verify_trust_anchor_signature(evidence, failures, signing_domain=SIGNING_DOMAIN)
    validate_controls(evidence.get("controls"), failures)
    validate_evidence_artifacts(
        evidence.get("evidence_artifacts"),
        failures,
        allowed_kinds=EVIDENCE_ARTIFACT_KINDS,
    )

    return {
        "provided": True,
        "path": str(path),
        "valid": not failures,
        "summary": {
            "schema_version": schema_version,
            "trust_anchor_type": trust_anchor_type,
            "key_attestor_key_id": key_attestor_key_id,
            "key_attestor_public_key_hex": key_attestor_public_key_hex,
            "key_attestor_ref": key_attestor_ref,
            "key_attestor_public_key_ref": key_attestor_public_key_ref,
            "publisher": publisher,
            "publisher_ref": publisher_ref,
            "publisher_key_id": publisher_key_id,
            "publisher_public_key_ref": publisher_public_key_ref,
            "publisher_public_key_hex": publisher_public_key_hex,
            "publication_ref": publication_ref,
            "signature_algorithm": signature_algorithm,
            "signature_ref": signature_ref,
            "signature_sha256_hex": signature_sha256_hex,
            "published_at": published_at,
            "valid_until": valid_until,
            "evidence_origin": origin["origin"],
        },
        "evidence_origin": origin["origin"],
        "synthetic_evidence": origin["synthetic"],
        "synthetic_evidence_reasons": origin["reasons"],
        "failures": failures,
    }


def validate_controls(raw: Any, failures: list[str]) -> None:
    validate_required_controls(
        raw,
        failures,
        field_name="controls",
        required_controls=REQUIRED_CONTROLS,
    )


def validate_external_reference(value: Any, failures: list[str], *, field_name: str) -> None:
    if not isinstance(value, str):
        return
    validate_non_local_reference(value, failures, field_name=field_name)
    if any(character.isspace() for character in value):
        failures.append(f"{field_name} must contain no raw whitespace")
