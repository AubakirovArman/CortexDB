#!/usr/bin/env python3
"""Validate operator-supplied external compliance certification evidence."""

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
    validate_not_expired,
    validate_not_future,
    validate_non_local_reference,
    validate_required_controls,
)


EVIDENCE_SCHEMA = "cortexdb.compliance_certification_evidence.v1"
COMPLIANCE_CERTIFICATION_EVIDENCE_FIELDS = {
    "schema_version",
    "framework",
    "certification_id",
    "issuer",
    "report_ref",
    "external_review",
    "scope",
    "immutability_evidence",
    "controls",
    "operator_responsibilities",
    "evidence_artifacts",
    "production_origin_proof",
}
EXTERNAL_REVIEW_FIELDS = {"reviewer", "assurance_level", "issued_at", "valid_until", "nda_required"}
COMPLIANCE_SCOPE_FIELDS = {"product", "version_range", "receipt_schema", "production_grade_public_receipts"}
IMMUTABILITY_EVIDENCE_FIELDS = {
    "external_immutable_store",
    "append_only_export",
    "tamper_evident_reviewed",
    "retention_days",
    "retention_policy_ref",
    "tamper_evidence_ref",
}
SUPPORTED_FRAMEWORKS = {"soc2_type_ii", "iso_27001"}
REQUIRED_CONTROLS = {
    "accountability_receipts_reviewed",
    "audit_log_immutability_reviewed",
    "transparency_log_operations_reviewed",
    "key_custody_boundary_reviewed",
    "operator_responsibility_matrix_reviewed",
}
REQUIRED_OPERATOR_RESPONSIBILITIES = {"operate the external immutable evidence store", "retain the redacted report under the evidence request process", "bind production receipt key custody evidence separately"}
EVIDENCE_ARTIFACT_KINDS = {"immutability_attestation", "redacted_external_report"}
NON_LOCAL_REFERENCE_FIELDS = {"report_ref", "immutability_evidence.retention_policy_ref", "immutability_evidence.tamper_evidence_ref"}


def validate_compliance_certification_evidence(
    path: Path,
    *,
    expected_framework: str | None = None,
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
    for field in sorted(set(evidence) - COMPLIANCE_CERTIFICATION_EVIDENCE_FIELDS):
        failures.append(f"compliance_certification_evidence.{field} is not allowed")

    schema_version = string_field(evidence, "schema_version", failures)
    if schema_version != EVIDENCE_SCHEMA:
        failures.append(f"schema_version must be {EVIDENCE_SCHEMA}")

    framework = string_field(evidence, "framework", failures)
    certification_id = string_field(evidence, "certification_id", failures)
    issuer = string_field(evidence, "issuer", failures)
    report_ref = string_field(evidence, "report_ref", failures)

    if framework not in SUPPORTED_FRAMEWORKS:
        failures.append(
            "framework must be one of " + ", ".join(sorted(SUPPORTED_FRAMEWORKS))
        )
    if expected_framework and framework != expected_framework:
        failures.append("framework does not match expected framework")
    if not certification_id or any(character.isspace() for character in certification_id):
        failures.append("certification_id must be non-empty and contain no whitespace")
    validate_external_reference(report_ref, failures, field_name="report_ref")

    review = validate_external_review(evidence.get("external_review"), failures)
    validate_scope(evidence.get("scope"), failures)
    validate_immutability(evidence.get("immutability_evidence"), failures)
    validate_controls(evidence.get("controls"), failures)
    validate_operator_responsibilities(evidence.get("operator_responsibilities"), failures)
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
            "framework": framework,
            "certification_id": certification_id,
            "issuer": issuer,
            "report_ref": report_ref,
            "assurance_level": review.get("assurance_level", ""),
            "valid_until": review.get("valid_until", ""),
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


def validate_external_review(raw: Any, failures: list[str]) -> dict[str, str]:
    if not isinstance(raw, dict):
        failures.append("external_review must be an object")
        return {}
    for field in sorted(set(raw) - EXTERNAL_REVIEW_FIELDS):
        failures.append(f"external_review.{field} is not allowed")
    reviewer = string_field(raw, "reviewer", failures)
    assurance_level = string_field(raw, "assurance_level", failures)
    issued_at = string_field(raw, "issued_at", failures)
    valid_until = string_field(raw, "valid_until", failures)
    if assurance_level not in {"limited", "reasonable", "certified"}:
        failures.append("external_review.assurance_level must be limited, reasonable, or certified")
    issued = parse_timestamp(issued_at, "external_review.issued_at", failures)
    valid = parse_timestamp(valid_until, "external_review.valid_until", failures)
    if issued and valid and valid <= issued:
        failures.append("external_review.valid_until must be after issued_at")
    validate_not_future(issued, failures, label="external_review.issued_at")
    validate_not_expired(valid, failures, label="external_review.valid_until")
    if bool_field(raw, "nda_required", failures) is not True:
        failures.append("external_review.nda_required must be true")
    return {
        "reviewer": reviewer,
        "assurance_level": assurance_level,
        "issued_at": issued_at,
        "valid_until": valid_until,
    }


def validate_scope(raw: Any, failures: list[str]) -> None:
    if not isinstance(raw, dict):
        failures.append("scope must be an object")
        return
    for field in sorted(set(raw) - COMPLIANCE_SCOPE_FIELDS):
        failures.append(f"scope.{field} is not allowed")
    if string_field(raw, "product", failures) != "CortexDB":
        failures.append("scope.product must be CortexDB")
    string_field(raw, "version_range", failures)
    if string_field(raw, "receipt_schema", failures) != "accountability_receipt.v1":
        failures.append("scope.receipt_schema must be accountability_receipt.v1")
    if bool_field(raw, "production_grade_public_receipts", failures) is not True:
        failures.append("scope.production_grade_public_receipts must be true")


def validate_immutability(raw: Any, failures: list[str]) -> None:
    if not isinstance(raw, dict):
        failures.append("immutability_evidence must be an object")
        return
    for field in sorted(set(raw) - IMMUTABILITY_EVIDENCE_FIELDS):
        failures.append(f"immutability_evidence.{field} is not allowed")
    if bool_field(raw, "external_immutable_store", failures) is not True:
        failures.append("immutability_evidence.external_immutable_store must be true")
    if bool_field(raw, "append_only_export", failures) is not True:
        failures.append("immutability_evidence.append_only_export must be true")
    if bool_field(raw, "tamper_evident_reviewed", failures) is not True:
        failures.append("immutability_evidence.tamper_evident_reviewed must be true")
    retention_days = raw.get("retention_days")
    if not isinstance(retention_days, int) or retention_days < 365:
        failures.append("immutability_evidence.retention_days must be an integer >= 365")
    validate_external_reference(
        string_field(raw, "retention_policy_ref", failures),
        failures,
        field_name="immutability_evidence.retention_policy_ref",
    )
    validate_external_reference(
        string_field(raw, "tamper_evidence_ref", failures),
        failures,
        field_name="immutability_evidence.tamper_evidence_ref",
    )


def validate_external_reference(value: str, failures: list[str], *, field_name: str) -> None:
    validate_non_local_reference(value, failures, field_name=field_name)
    if any(character.isspace() for character in value):
        failures.append(f"{field_name} must contain no raw whitespace")


def validate_controls(raw: Any, failures: list[str]) -> None:
    validate_required_controls(
        raw,
        failures,
        field_name="controls",
        required_controls=REQUIRED_CONTROLS,
    )


def validate_operator_responsibilities(raw: Any, failures: list[str]) -> None:
    if not isinstance(raw, list) or not raw or not all(isinstance(item, str) and item.strip() for item in raw):
        failures.append("operator_responsibilities must be a non-empty string list")
        return
    supplied = set(raw)
    missing = sorted(REQUIRED_OPERATOR_RESPONSIBILITIES.difference(supplied))
    if missing:
        failures.append(f"operator_responsibilities missing required entries: {', '.join(missing)}")
    unsupported = sorted(supplied.difference(REQUIRED_OPERATOR_RESPONSIBILITIES))
    if unsupported:
        failures.append(f"operator_responsibilities contains unsupported entries: {', '.join(unsupported)}")
    duplicates = sorted({item for item in raw if raw.count(item) > 1})
    if duplicates:
        failures.append(f"operator_responsibilities contains duplicate entries: {', '.join(duplicates)}")

