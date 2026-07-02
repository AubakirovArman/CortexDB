#!/usr/bin/env python3
"""Validate operator-supplied receipt KMS/HSM custody evidence."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from evidence_origin import (
    classify_evidence_origin,
    production_origin_proof_failures,
    production_origin_proof_summary,
)
from operator_evidence_validation import (
    FORBIDDEN_SECRET_KEYS,
    forbidden_secret_paths,
    parse_timestamp,
    validate_evidence_artifacts,
    validate_not_future,
    validate_not_expired,
    validate_non_local_reference,
    validate_required_controls,
)
from receipt_kms_hsm_runtime_probe import (
    REQUEST_SCHEMA,
    RESPONSE_SCHEMA,
    RUNTIME_SIGNING_PROBE_FIELDS,
    RUNTIME_SIGNING_PROBE_TYPE,
    SIGNING_DOMAIN,
    validate_runtime_signing_probe,
)


EVIDENCE_SCHEMA = "cortexdb.receipt_kms_hsm_custody_evidence.v1"
KMS_HSM_CUSTODY_EVIDENCE_FIELDS = {
    "schema_version",
    "custody_mode",
    "provider",
    "provider_key_ref",
    "signer_ref",
    "key_id",
    "public_key_hex",
    "signing_domain",
    "key_material_exportable",
    "local_seed_material_present",
    "runtime_binding",
    "runtime_signing_probe",
    "operator_attestation",
    "evidence_artifacts",
    "production_origin_proof",
}
RUNTIME_BINDING_FIELDS = {
    "mode",
    "local_seed_fallback",
    "request_schema",
    "response_schema",
}
OPERATOR_ATTESTATION_FIELDS = {
    "attestation_id",
    "operator_id",
    "issued_at",
    "valid_until",
    "controls",
}

REQUIRED_CONTROLS = {
    "kms_or_hsm_key_is_non_exportable",
    "receipt_signer_uses_provider_key_ref",
    "local_seed_disabled_for_production",
    "public_key_bound_to_provider_key",
}
EVIDENCE_ARTIFACT_KINDS = {"provider_key_policy", "signer_deployment_config"}
NON_LOCAL_REFERENCE_FIELDS = {"provider_key_ref", "signer_ref"}


def validate_operator_custody_evidence(
    path: Path,
    *,
    expected_key_id: str | None = None,
    expected_public_key_hex: str | None = None,
    expected_signer_ref: str | None = None,
    expected_key_attestor_key_id: str | None = None,
    expected_key_attestor_public_key_hex: str | None = None,
    expected_key_attestor_ref: str | None = None,
    expected_key_attestor_public_key_ref: str | None = None,
    require_production_origin_proof: bool = False,
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
    proof_failures = production_origin_proof_failures(
        evidence,
        expected_key_attestor_key_id=expected_key_attestor_key_id,
        expected_key_attestor_public_key_hex=expected_key_attestor_public_key_hex,
        expected_key_attestor_ref=expected_key_attestor_ref,
        expected_key_attestor_public_key_ref=expected_key_attestor_public_key_ref,
    )
    if require_production_origin_proof:
        failures.extend(proof_failures)
    for field_path in forbidden_secret_paths(evidence):
        failures.append(f"forbidden secret material field present: {field_path}")
    for field in sorted(set(evidence) - KMS_HSM_CUSTODY_EVIDENCE_FIELDS):
        failures.append(f"kms_hsm_custody_evidence.{field} is not allowed")

    schema_version = string_field(evidence, "schema_version", failures)
    if schema_version != EVIDENCE_SCHEMA:
        failures.append(f"schema_version must be {EVIDENCE_SCHEMA}")

    custody_mode = string_field(evidence, "custody_mode", failures)
    if custody_mode not in {"kms", "hsm", "kms_hsm"}:
        failures.append("custody_mode must be one of kms, hsm, kms_hsm")

    provider = string_field(evidence, "provider", failures)
    provider_key_ref = string_field(evidence, "provider_key_ref", failures)
    key_id = string_field(evidence, "key_id", failures)
    signer_ref = string_field(evidence, "signer_ref", failures)
    public_key_hex = string_field(evidence, "public_key_hex", failures)
    signing_domain = string_field(evidence, "signing_domain", failures)

    if not key_id or any(character.isspace() for character in key_id):
        failures.append("key_id must be non-empty and contain no whitespace")
    for field_name, value in (
        ("provider_key_ref", provider_key_ref),
        ("signer_ref", signer_ref),
    ):
        if any(character.isspace() for character in value):
            failures.append(f"{field_name} must contain no whitespace")
    if not is_hex(public_key_hex, 64):
        failures.append("public_key_hex must be 64 lowercase hex characters")
    if signing_domain != SIGNING_DOMAIN:
        failures.append(f"signing_domain must be {SIGNING_DOMAIN}")
    if evidence.get("key_material_exportable") is not False:
        failures.append("key_material_exportable must be false")
    if evidence.get("local_seed_material_present") is not False:
        failures.append("local_seed_material_present must be false")
    if expected_key_id and key_id != expected_key_id:
        failures.append("key_id does not match expected runtime key id")
    if expected_public_key_hex:
        if not is_hex(expected_public_key_hex, 64):
            failures.append("expected runtime public key must be 64 lowercase hex characters")
        elif public_key_hex != expected_public_key_hex:
            failures.append("public_key_hex does not match expected runtime public key")
    if expected_signer_ref and signer_ref != expected_signer_ref:
        failures.append("signer_ref does not match expected runtime signer ref")

    validate_non_local_reference(provider_key_ref, failures, field_name="provider_key_ref")
    validate_non_local_reference(signer_ref, failures, field_name="signer_ref")
    validate_provider_ref(custody_mode, provider_key_ref, failures)
    validate_runtime_binding(evidence.get("runtime_binding"), failures)
    runtime_signing_probe = validate_runtime_signing_probe(
        evidence.get("runtime_signing_probe"),
        key_id=key_id,
        public_key_hex=public_key_hex,
        signer_ref=signer_ref,
        failures=failures,
        require_fresh_signed_at=origin["origin"] == "operator" and not origin["synthetic"],
    )
    validate_attestation(evidence.get("operator_attestation"), failures)
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
            "custody_mode": custody_mode,
            "provider": provider,
            "provider_key_ref": provider_key_ref,
            "key_id": key_id,
            "public_key_hex": public_key_hex,
            "signer_ref": signer_ref,
            "signing_domain": signing_domain,
            "runtime_signing_probe": runtime_signing_probe,
            "evidence_origin": origin["origin"],
        },
        "evidence_origin": origin["origin"],
        "synthetic_evidence": origin["synthetic"],
        "synthetic_evidence_reasons": origin["reasons"],
        "production_origin_proof_required": require_production_origin_proof,
        "production_origin_proof": production_origin_proof_summary(evidence),
        "production_origin_proof_valid": not proof_failures,
        "failures": failures,
    }


def invalid_result(path: Path, failures: list[str]) -> dict[str, Any]:
    return {
        "provided": True,
        "path": str(path),
        "valid": False,
        "summary": {},
        "evidence_origin": "unknown",
        "synthetic_evidence": False,
        "synthetic_evidence_reasons": [],
        "production_origin_proof_required": False,
        "production_origin_proof": {"provided": False},
        "production_origin_proof_valid": False,
        "failures": failures,
    }


def string_field(value: dict[str, Any], name: str, failures: list[str]) -> str:
    raw = value.get(name)
    if isinstance(raw, str) and raw.strip():
        if raw != raw.strip():
            failures.append(f"{name} must not include surrounding whitespace")
        return raw.strip()
    failures.append(f"{name} must be a non-empty string")
    return ""


def bool_field(value: dict[str, Any], name: str, failures: list[str]) -> bool | None:
    raw = value.get(name)
    if isinstance(raw, bool):
        return raw
    failures.append(f"{name} must be a boolean")
    return None


def is_hex(value: str, length: int) -> bool:
    return len(value) == length and all(character in "0123456789abcdef" for character in value)


def validate_provider_ref(custody_mode: str, provider_key_ref: str, failures: list[str]) -> None:
    allowed_prefixes = {
        "kms": ("kms://", "arn:"),
        "hsm": ("hsm://", "pkcs11:"),
        "kms_hsm": ("kms://", "hsm://", "pkcs11:", "arn:"),
    }.get(custody_mode, ())
    if provider_key_ref and allowed_prefixes and not provider_key_ref.startswith(allowed_prefixes):
        failures.append("provider_key_ref prefix does not match custody_mode")


def validate_runtime_binding(raw: Any, failures: list[str]) -> None:
    if not isinstance(raw, dict):
        failures.append("runtime_binding must be an object")
        return
    for field in sorted(set(raw) - RUNTIME_BINDING_FIELDS):
        failures.append(f"runtime_binding.{field} is not allowed")
    if string_field(raw, "mode", failures) != "external_command":
        failures.append("runtime_binding.mode must be external_command")
    if bool_field(raw, "local_seed_fallback", failures) is not False:
        failures.append("runtime_binding.local_seed_fallback must be false")
    if string_field(raw, "request_schema", failures) != REQUEST_SCHEMA:
        failures.append(f"runtime_binding.request_schema must be {REQUEST_SCHEMA}")
    if string_field(raw, "response_schema", failures) != RESPONSE_SCHEMA:
        failures.append(f"runtime_binding.response_schema must be {RESPONSE_SCHEMA}")


def validate_attestation(raw: Any, failures: list[str]) -> None:
    if not isinstance(raw, dict):
        failures.append("operator_attestation must be an object")
        return
    for field in sorted(set(raw) - OPERATOR_ATTESTATION_FIELDS):
        failures.append(f"operator_attestation.{field} is not allowed")
    string_field(raw, "attestation_id", failures)
    string_field(raw, "operator_id", failures)
    issued_at = parse_timestamp(string_field(raw, "issued_at", failures), "operator_attestation.issued_at", failures)
    valid_until = parse_timestamp(string_field(raw, "valid_until", failures), "operator_attestation.valid_until", failures)
    if issued_at and valid_until and valid_until <= issued_at:
        failures.append("operator_attestation.valid_until must be after issued_at")
    validate_not_future(issued_at, failures, label="operator_attestation.issued_at")
    validate_not_expired(valid_until, failures, label="operator_attestation.valid_until")
    validate_required_controls(
        raw.get("controls"),
        failures,
        field_name="operator_attestation.controls",
        required_controls=REQUIRED_CONTROLS,
    )

