#!/usr/bin/env python3
"""Validate production evidence handoff consistency with validator constants."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from compliance_certification_evidence import (
    COMPLIANCE_CERTIFICATION_EVIDENCE_FIELDS,
    COMPLIANCE_SCOPE_FIELDS,
    EVIDENCE_ARTIFACT_KINDS as COMPLIANCE_EVIDENCE_ARTIFACT_KINDS,
    EVIDENCE_SCHEMA as COMPLIANCE_EVIDENCE_SCHEMA,
    EXTERNAL_REVIEW_FIELDS,
    FORBIDDEN_SECRET_KEYS as COMPLIANCE_FORBIDDEN_SECRET_KEYS,
    IMMUTABILITY_EVIDENCE_FIELDS,
    NON_LOCAL_REFERENCE_FIELDS as COMPLIANCE_NON_LOCAL_REFERENCE_FIELDS,
    REQUIRED_CONTROLS as COMPLIANCE_REQUIRED_CONTROLS,
    REQUIRED_OPERATOR_RESPONSIBILITIES as COMPLIANCE_REQUIRED_OPERATOR_RESPONSIBILITIES,
    SUPPORTED_FRAMEWORKS,
)
from evidence_origin import (
    PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
    PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
    PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
    PRODUCTION_ORIGIN_PROOF_SCHEMA,
    PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
    PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
    PRODUCTION_ORIGIN_STATEMENT_TYPE,
)
from operator_evidence_validation import EVIDENCE_ARTIFACT_FIELDS
from receipt_kms_hsm_evidence import (
    EVIDENCE_ARTIFACT_KINDS as KMS_HSM_EVIDENCE_ARTIFACT_KINDS,
    EVIDENCE_SCHEMA as KMS_HSM_EVIDENCE_SCHEMA,
    FORBIDDEN_SECRET_KEYS as KMS_HSM_FORBIDDEN_SECRET_KEYS,
    KMS_HSM_CUSTODY_EVIDENCE_FIELDS,
    NON_LOCAL_REFERENCE_FIELDS as KMS_HSM_NON_LOCAL_REFERENCE_FIELDS,
    OPERATOR_ATTESTATION_FIELDS,
    REQUEST_SCHEMA,
    REQUIRED_CONTROLS as KMS_HSM_REQUIRED_CONTROLS,
    RESPONSE_SCHEMA,
    RUNTIME_BINDING_FIELDS,
    RUNTIME_SIGNING_PROBE_FIELDS,
    RUNTIME_SIGNING_PROBE_TYPE,
    SIGNING_DOMAIN,
)
from receipt_production_evidence_handoff_payload import operator_handoff
from receipt_production_evidence_preflight import REQUIRED_INPUTS
from receipt_production_origin_trust_anchor_evidence import (
    EVIDENCE_ARTIFACT_KINDS as TRUST_ANCHOR_EVIDENCE_ARTIFACT_KINDS,
    EVIDENCE_SCHEMA as TRUST_ANCHOR_EVIDENCE_SCHEMA,
    FORBIDDEN_SECRET_KEYS as TRUST_ANCHOR_FORBIDDEN_SECRET_KEYS,
    NON_LOCAL_REFERENCE_FIELDS as TRUST_ANCHOR_NON_LOCAL_REFERENCE_FIELDS,
    REQUIRED_CONTROLS as TRUST_ANCHOR_REQUIRED_CONTROLS,
    SIGNING_DOMAIN as TRUST_ANCHOR_SIGNING_DOMAIN,
    TRUST_ANCHOR_EVIDENCE_FIELDS,
    TRUST_ANCHOR_TYPE,
)


def build_report() -> dict[str, Any]:
    handoff = operator_handoff()
    failures: list[str] = []
    require(handoff.get("schema_version") == "cortexdb.receipt_production_evidence_handoff.v1", failures, "unexpected handoff schema")
    require("status" not in handoff, failures, "handoff must not emit status")
    require("production_evidence_ready" not in handoff, failures, "handoff must not emit readiness")
    check_required_inputs(handoff, failures)
    check_schemas(handoff, failures)
    check_checklist(handoff, failures)
    check_origin_boundary(handoff, failures)
    return {
        "schema_version": "cortexdb.receipt_production_evidence_handoff_check.v1",
        "status": "passed" if not failures else "failed",
        "checked_inputs": list(REQUIRED_INPUTS.values()),
        "checked_components": [
            "production_origin_trust_anchor",
            "receipt_kms_hsm_custody",
            "compliance_certification",
        ],
        "failures": failures,
        "claim_boundary": (
            "consistency check only; does not validate, synthesize, or replace "
            "operator KMS/HSM custody or external compliance evidence"
        ),
    }


def require(condition: bool, failures: list[str], message: str) -> None:
    if not condition:
        failures.append(message)


def check_required_inputs(handoff: dict[str, Any], failures: list[str]) -> None:
    raw_inputs = handoff.get("required_inputs")
    require(isinstance(raw_inputs, list), failures, "required_inputs must be a list")
    envs = [item.get("env") for item in raw_inputs if isinstance(item, dict)]
    require(envs == list(REQUIRED_INPUTS.values()), failures, "required_inputs env order drifted from preflight")
    by_env = {item.get("env"): item for item in raw_inputs if isinstance(item, dict)}
    for env in (
        "RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX",
        "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX",
        "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX",
    ):
        require(
            by_env.get(env, {}).get("format") == "64 lowercase hex characters",
            failures,
            f"{env} lowercase hex input contract missing",
        )


def check_schemas(handoff: dict[str, Any], failures: list[str]) -> None:
    schemas = handoff.get("evidence_schemas")
    if not isinstance(schemas, dict):
        failures.append("evidence_schemas must be an object")
        return
    expected = {
        "receipt_kms_hsm_custody": KMS_HSM_EVIDENCE_SCHEMA,
        "compliance_certification": COMPLIANCE_EVIDENCE_SCHEMA,
        "production_origin_trust_anchor": TRUST_ANCHOR_EVIDENCE_SCHEMA,
        "production_origin_proof": PRODUCTION_ORIGIN_PROOF_SCHEMA,
        "production_origin_statement": PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
        "production_origin_statement_signing_domain": PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        "production_origin_key_attestation": PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
        "production_origin_key_attestation_signing_domain": PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        "external_sign_request": REQUEST_SCHEMA,
        "external_signature_response": RESPONSE_SCHEMA,
        "signing_domain": SIGNING_DOMAIN,
    }
    require(schemas == expected, failures, "evidence_schemas drifted from validator constants")


def check_checklist(handoff: dict[str, Any], failures: list[str]) -> None:
    checklist = handoff.get("evidence_field_checklist")
    if not isinstance(checklist, dict):
        failures.append("evidence_field_checklist must be an object")
        return
    kms = checklist.get("receipt_kms_hsm_custody")
    compliance = checklist.get("compliance_certification")
    trust_anchor = checklist.get("production_origin_trust_anchor")
    if not isinstance(trust_anchor, dict):
        failures.append("production_origin_trust_anchor checklist must be an object")
    else:
        check_trust_anchor_checklist(trust_anchor, failures)
    if not isinstance(kms, dict):
        failures.append("receipt_kms_hsm_custody checklist must be an object")
    else:
        check_kms_checklist(kms, failures)
    if not isinstance(compliance, dict):
        failures.append("compliance_certification checklist must be an object")
    else:
        check_compliance_checklist(compliance, failures)


def check_trust_anchor_checklist(trust_anchor: dict[str, Any], failures: list[str]) -> None:
    values = trust_anchor.get("required_values", {})
    require(
        trust_anchor.get("schema_version") == TRUST_ANCHOR_EVIDENCE_SCHEMA,
        failures,
        "trust anchor schema checklist drifted",
    )
    require(
        values.get("trust_anchor_type") == TRUST_ANCHOR_TYPE,
        failures,
        "trust anchor type checklist drifted",
    )
    require(
        values.get("external_control_plane") is True,
        failures,
        "trust anchor external control plane checklist drifted",
    )
    require(
        values.get("key_attestor_binding"),
        failures,
        "trust anchor key attestor binding guidance missing",
    )
    require(
        values.get("publisher_binding"),
        failures,
        "trust anchor publisher binding guidance missing",
    )
    require(
        "non-local" in str(values.get("reference_values", ""))
        and "whitespace" in str(values.get("reference_values", "")),
        failures,
        "trust anchor reference canonicality guidance missing",
    )
    require(
        "whitespace" in str(values.get("key_ids", "")),
        failures,
        "trust anchor key id canonicality guidance missing",
    )
    require(
        values.get("signature_algorithm") == "ed25519",
        failures,
        "trust anchor signature algorithm checklist drifted",
    )
    require(
        values.get("signature_hex"),
        failures,
        "trust anchor signature hex guidance missing",
    )
    for key in ("published_at", "valid_until"):
        require(
            "timezone-aware" in str(values.get(key, "")),
            failures,
            f"trust anchor {key} timezone guidance missing",
        )
    for key in (
        "key_attestor_public_key_hex",
        "publisher_public_key_hex",
        "signature_hex",
        "signature_sha256_hex",
    ):
        require(
            "lowercase" in str(values.get(key, "")) and "hex" in str(values.get(key, "")) and "surrounding whitespace" in str(values.get(key, "")),
            failures,
            f"trust anchor {key} lowercase hex guidance missing",
        )
    require(
        trust_anchor.get("signing_domain") == TRUST_ANCHOR_SIGNING_DOMAIN,
        failures,
        "trust anchor signing domain drifted",
    )
    require(
        trust_anchor.get("closed_shape_fields") == sorted(TRUST_ANCHOR_EVIDENCE_FIELDS),
        failures,
        "trust anchor closed-shape fields drifted",
    )
    require(
        trust_anchor.get("non_local_reference_fields") == sorted(TRUST_ANCHOR_NON_LOCAL_REFERENCE_FIELDS),
        failures,
        "trust anchor non-local reference checklist drifted",
    )
    for field in (
        "key_attestor_key_id",
        "key_attestor_public_key_hex",
        "key_attestor_ref",
        "key_attestor_public_key_ref",
        "publication_ref",
        "publisher_ref",
        "publisher_key_id",
        "publisher_public_key_ref",
        "publisher_public_key_hex",
        "signature_algorithm",
        "signature_ref",
        "signature_hex",
        "signature_sha256_hex",
        "published_at",
        "valid_until",
    ):
        require(
            field in trust_anchor.get("required_top_level_fields", []),
            failures,
            f"trust anchor {field} missing",
        )
    require(
        trust_anchor.get("required_controls") == sorted(TRUST_ANCHOR_REQUIRED_CONTROLS),
        failures,
        "trust anchor controls checklist drifted",
    )
    check_controls_contract(trust_anchor, "trust anchor", failures)
    require(
        trust_anchor.get("forbidden_secret_fields") == sorted(TRUST_ANCHOR_FORBIDDEN_SECRET_KEYS),
        failures,
        "trust anchor forbidden secrets checklist drifted",
    )
    check_forbidden_secret_matching(trust_anchor, "trust anchor", failures)
    check_artifacts(
        trust_anchor,
        "trust anchor",
        failures,
        expected_kinds=TRUST_ANCHOR_EVIDENCE_ARTIFACT_KINDS,
    )


def check_kms_checklist(kms: dict[str, Any], failures: list[str]) -> None:
    values = kms.get("required_values", {})
    require(values.get("schema_version") == KMS_HSM_EVIDENCE_SCHEMA, failures, "KMS/HSM schema checklist drifted")
    require(values.get("signing_domain") == SIGNING_DOMAIN, failures, "KMS/HSM signing domain checklist drifted")
    require(values.get("runtime_binding.request_schema") == REQUEST_SCHEMA, failures, "KMS/HSM request schema checklist drifted")
    require(values.get("runtime_binding.response_schema") == RESPONSE_SCHEMA, failures, "KMS/HSM response schema checklist drifted")
    require(
        "runtime_signing_probe" in kms.get("required_top_level_fields", []),
        failures,
        "KMS/HSM runtime signing probe missing from top-level checklist",
    )
    require(
        values.get("runtime_signing_probe.probe_type") == RUNTIME_SIGNING_PROBE_TYPE,
        failures,
        "KMS/HSM runtime signing probe type checklist drifted",
    )
    require(
        values.get("runtime_signing_probe.request_schema") == REQUEST_SCHEMA,
        failures,
        "KMS/HSM runtime signing probe request schema checklist drifted",
    )
    require(
        values.get("runtime_signing_probe.response_schema") == RESPONSE_SCHEMA,
        failures,
        "KMS/HSM runtime signing probe response schema checklist drifted",
    )
    require(
        values.get("runtime_signing_probe.signing_domain") == SIGNING_DOMAIN,
        failures,
        "KMS/HSM runtime signing probe signing domain checklist drifted",
    )
    for key in (
        "public_key_hex",
        "runtime_signing_probe.public_key_hex",
        "runtime_signing_probe.canonical_header_hex",
        "runtime_signing_probe.request_sha256_hex",
        "runtime_signing_probe.response_sha256_hex",
        "runtime_signing_probe.signature_hex",
        "runtime_signing_probe.signature_sha256_hex",
    ):
        require(
            "lowercase" in str(values.get(key, "")) and "hex" in str(values.get(key, "")) and "surrounding whitespace" in str(values.get(key, "")),
            failures,
            f"KMS/HSM {key} lowercase hex guidance missing",
        )
    require(values.get("runtime_signing_probe.key_binding"), failures, "KMS/HSM runtime signing probe key binding guidance missing")
    for key in ("provider_key_ref", "signer_ref", "runtime_signing_probe.key_binding"):
        require(
            "whitespace" in str(values.get(key, "")),
            failures,
            f"KMS/HSM {key} whitespace guidance missing",
        )
    require(values.get("runtime_signing_probe.signature_hex"), failures, "KMS/HSM runtime signing probe signature guidance missing")
    for key in (
        "runtime_signing_probe.signed_at",
        "operator_attestation.issued_at",
        "operator_attestation.valid_until",
    ):
        require(
            "timezone-aware" in str(values.get(key, "")),
            failures,
            f"KMS/HSM {key} timezone guidance missing",
        )
    require(kms.get("required_controls") == sorted(KMS_HSM_REQUIRED_CONTROLS), failures, "KMS/HSM controls checklist drifted")
    check_controls_contract(kms, "KMS/HSM", failures)
    require(kms.get("non_local_reference_fields") == sorted(KMS_HSM_NON_LOCAL_REFERENCE_FIELDS), failures, "KMS/HSM non-local reference checklist drifted")
    require(kms.get("forbidden_secret_fields") == sorted(KMS_HSM_FORBIDDEN_SECRET_KEYS), failures, "KMS/HSM forbidden secrets checklist drifted")
    check_forbidden_secret_matching(kms, "KMS/HSM", failures)
    require(kms.get("closed_shape_fields") == sorted(KMS_HSM_CUSTODY_EVIDENCE_FIELDS), failures, "KMS/HSM closed-shape fields drifted")
    expected_nested = {"runtime_binding": sorted(RUNTIME_BINDING_FIELDS), "runtime_signing_probe": sorted(RUNTIME_SIGNING_PROBE_FIELDS), "operator_attestation": sorted(OPERATOR_ATTESTATION_FIELDS)}
    require(kms.get("nested_closed_shape_fields") == expected_nested, failures, "KMS/HSM nested closed-shape fields drifted")
    check_production_origin_proof(kms, "KMS/HSM", failures)
    check_artifacts(kms, "KMS/HSM", failures, expected_kinds=KMS_HSM_EVIDENCE_ARTIFACT_KINDS)


def check_compliance_checklist(compliance: dict[str, Any], failures: list[str]) -> None:
    values = compliance.get("required_values", {})
    require(values.get("schema_version") == COMPLIANCE_EVIDENCE_SCHEMA, failures, "compliance schema checklist drifted")
    require(values.get("framework") == sorted(SUPPORTED_FRAMEWORKS), failures, "compliance framework checklist drifted")
    require("surrounding whitespace" in str(values.get("string_values", "")), failures, "compliance string canonicalization checklist drifted")
    require(compliance.get("required_controls") == sorted(COMPLIANCE_REQUIRED_CONTROLS), failures, "compliance controls checklist drifted")
    check_controls_contract(compliance, "compliance", failures)
    require(
        compliance.get("required_operator_responsibilities") == sorted(COMPLIANCE_REQUIRED_OPERATOR_RESPONSIBILITIES),
        failures,
        "compliance operator responsibilities checklist drifted",
    )
    require(
        compliance.get("operator_responsibilities_closed_set") is True,
        failures,
        "compliance operator responsibilities closed-set rule drifted",
    )
    require(
        compliance.get("operator_responsibilities_duplicate_free") is True,
        failures,
        "compliance operator responsibilities duplicate-free rule drifted",
    )
    require(
        compliance.get("non_local_reference_fields") == sorted(COMPLIANCE_NON_LOCAL_REFERENCE_FIELDS),
        failures,
        "compliance non-local reference checklist drifted",
    )
    for key in (
        "report_ref",
        "immutability_evidence.retention_policy_ref",
        "immutability_evidence.tamper_evidence_ref",
    ):
        require(
            "whitespace" in str(values.get(key, "")),
            failures,
            f"compliance {key} whitespace guidance missing",
        )
    for key in ("external_review.issued_at", "external_review.valid_until"):
        require(
            "timezone-aware" in str(values.get(key, "")),
            failures,
            f"compliance {key} timezone guidance missing",
        )
    require(compliance.get("forbidden_secret_fields") == sorted(COMPLIANCE_FORBIDDEN_SECRET_KEYS), failures, "compliance forbidden secrets checklist drifted")
    check_forbidden_secret_matching(compliance, "compliance", failures)
    require(compliance.get("closed_shape_fields") == sorted(COMPLIANCE_CERTIFICATION_EVIDENCE_FIELDS), failures, "compliance closed-shape fields drifted")
    expected_nested = {"external_review": sorted(EXTERNAL_REVIEW_FIELDS), "scope": sorted(COMPLIANCE_SCOPE_FIELDS), "immutability_evidence": sorted(IMMUTABILITY_EVIDENCE_FIELDS)}
    require(compliance.get("nested_closed_shape_fields") == expected_nested, failures, "compliance nested closed-shape fields drifted")
    check_production_origin_proof(compliance, "compliance", failures)
    check_artifacts(
        compliance,
        "compliance",
        failures,
        expected_kinds=COMPLIANCE_EVIDENCE_ARTIFACT_KINDS,
    )


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


def check_artifacts(
    component: dict[str, Any],
    label: str,
    failures: list[str],
    *,
    expected_kinds: set[str],
) -> None:
    artifacts = component.get("artifact_requirements")
    if not isinstance(artifacts, dict):
        failures.append(f"{label} artifact requirements must be an object")
        return
    require(artifacts.get("minimum_count") == 2, failures, f"{label} artifact minimum count drifted")
    require(artifacts.get("required_fields") == list(EVIDENCE_ARTIFACT_FIELDS), failures, f"{label} artifact fields drifted")
    require(
        artifacts.get("allowed_kinds") == sorted(expected_kinds),
        failures,
        f"{label} artifact kind closed-set drifted",
    )
    require(
        artifacts.get("closed_shape") == "no fields beyond required_fields are accepted",
        failures,
        f"{label} artifact closed-shape rule drifted",
    )
    require(
        "non-local" in str(artifacts.get("uri", ""))
        and "whitespace" in str(artifacts.get("uri", "")),
        failures,
        f"{label} artifact URI boundary guidance missing",
    )
    require(
        "lowercase" in str(artifacts.get("sha256_hex", ""))
        and "hex" in str(artifacts.get("sha256_hex", ""))
        and "surrounding whitespace" in str(artifacts.get("sha256_hex", "")),
        failures,
        f"{label} artifact digest lowercase hex whitespace guidance missing",
    )


def check_forbidden_secret_matching(
    component: dict[str, Any],
    label: str,
    failures: list[str],
) -> None:
    guidance = str(component.get("forbidden_secret_field_name_matching", ""))
    require(
        "case-insensitive" in guidance
        and "normalized" in guidance
        and "snake_case" in guidance
        and "camelCase" in guidance
        and "kebab-case" in guidance,
        failures,
        f"{label} forbidden secret field-name matching guidance missing",
    )


def check_controls_contract(component: dict[str, Any], label: str, failures: list[str]) -> None:
    require(component.get("controls_closed_set") is True, failures, f"{label} controls closed-set rule drifted")
    require(component.get("controls_duplicate_free") is True, failures, f"{label} controls duplicate-free rule drifted")


def check_origin_boundary(handoff: dict[str, Any], failures: list[str]) -> None:
    boundary = handoff.get("origin_boundary")
    if not isinstance(boundary, dict):
        failures.append("origin_boundary must be an object")
        return
    require(boundary.get("accepted_origin") == "operator", failures, "accepted origin must remain operator")
    rejected = set(boundary.get("rejected_origins", []))
    for origin in {"generated_local_artifact", "temporary_local_artifact", "local_reference_artifact", "synthetic_fixture", "missing", "unknown"}:
        require(origin in rejected, failures, f"missing rejected origin {origin}")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    report = build_report()
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"receipt production evidence handoff check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
