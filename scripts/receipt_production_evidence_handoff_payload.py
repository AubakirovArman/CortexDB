"""Build machine-readable production evidence handoff payloads."""

from __future__ import annotations

from typing import Any

from compliance_certification_evidence import COMPLIANCE_CERTIFICATION_EVIDENCE_FIELDS, COMPLIANCE_SCOPE_FIELDS, EVIDENCE_ARTIFACT_KINDS as COMPLIANCE_EVIDENCE_ARTIFACT_KINDS, EVIDENCE_SCHEMA as COMPLIANCE_EVIDENCE_SCHEMA, EXTERNAL_REVIEW_FIELDS, FORBIDDEN_SECRET_KEYS as COMPLIANCE_FORBIDDEN_SECRET_KEYS, IMMUTABILITY_EVIDENCE_FIELDS, NON_LOCAL_REFERENCE_FIELDS as COMPLIANCE_NON_LOCAL_REFERENCE_FIELDS, REQUIRED_CONTROLS as COMPLIANCE_REQUIRED_CONTROLS, REQUIRED_OPERATOR_RESPONSIBILITIES as COMPLIANCE_REQUIRED_OPERATOR_RESPONSIBILITIES, SUPPORTED_FRAMEWORKS
from evidence_origin import (
    PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
    PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
    PRODUCTION_ORIGIN_PROOF_SCHEMA,
    PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
    PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
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
from receipt_production_origin_trust_anchor_evidence import EVIDENCE_SCHEMA as TRUST_ANCHOR_EVIDENCE_SCHEMA
from receipt_production_evidence_handoff_requirements import origin_boundary, production_origin_proof_requirements, production_origin_trust_anchor_requirements


def operator_handoff() -> dict[str, Any]:
    return {
        "schema_version": "cortexdb.receipt_production_evidence_handoff.v1",
        "purpose": (
            "operator-supplied evidence required before production-grade public "
            "receipt claims"
        ),
        "required_inputs": required_inputs(),
        "evidence_schemas": evidence_schemas(),
        "evidence_field_checklist": evidence_field_checklist(),
        "origin_boundary": origin_boundary(),
        "validation_command": (
            "make receipt-production-ready-check "
            "RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<operator-json> "
            "RECEIPT_KMS_HSM_EXPECTED_KEY_ID=<runtime-key-id> "
            "RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX=<runtime-public-key-hex> "
            "RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF=<runtime-signer-ref> "
            "COMPLIANCE_CERTIFICATION_EVIDENCE=<operator-json> "
            "COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK=<soc2_type_ii|iso_27001> "
            "RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=<operator-json> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID=<attestor-key-id> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX=<attestor-public-key-hex> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF=<attestor-ref> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF=<attestor-public-key-ref> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID=<publisher-key-id> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX=<publisher-public-key-hex> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF=<publisher-ref> "
            "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF=<publisher-public-key-ref>"
        ),
        "claim_boundary": "handoff only; does not create or replace operator KMS/HSM custody or external compliance evidence",
    }


def required_inputs() -> list[dict[str, Any]]:
    return [
        {
            "env": "RECEIPT_KMS_HSM_CUSTODY_EVIDENCE",
            "kind": "json_path",
            "schema_version": KMS_HSM_EVIDENCE_SCHEMA,
        },
        {
            "env": "RECEIPT_KMS_HSM_EXPECTED_KEY_ID",
            "kind": "runtime_binding",
            "binds": "receipt signer key_id",
        },
        {
            "env": "RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX",
            "kind": "runtime_binding",
            "binds": "receipt signer public_key_hex",
            "format": "64 lowercase hex characters",
        },
        {
            "env": "RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF",
            "kind": "runtime_binding",
            "binds": "receipt external signer reference",
        },
        {
            "env": "COMPLIANCE_CERTIFICATION_EVIDENCE",
            "kind": "json_path",
            "schema_version": COMPLIANCE_EVIDENCE_SCHEMA,
        },
        {
            "env": "COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK",
            "kind": "runtime_binding",
            "allowed_values": sorted(SUPPORTED_FRAMEWORKS),
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE",
            "kind": "json_path",
            "schema_version": TRUST_ANCHOR_EVIDENCE_SCHEMA,
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID",
            "kind": "production_origin_trust_anchor",
            "binds": "production_origin_proof.key_attestor_key_id",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX",
            "kind": "production_origin_trust_anchor",
            "binds": "production_origin_proof.key_attestor_public_key_hex",
            "format": "64 lowercase hex characters",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF",
            "kind": "production_origin_trust_anchor",
            "binds": "production_origin_proof.key_attestor_ref",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF",
            "kind": "production_origin_trust_anchor",
            "binds": "production_origin_proof.key_attestor_public_key_ref",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID",
            "kind": "production_origin_trust_anchor_publisher",
            "binds": "production_origin_trust_anchor.publisher_key_id",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX",
            "kind": "production_origin_trust_anchor_publisher",
            "binds": "production_origin_trust_anchor.publisher_public_key_hex",
            "format": "64 lowercase hex characters",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF",
            "kind": "production_origin_trust_anchor_publisher",
            "binds": "production_origin_trust_anchor.publisher_ref",
        },
        {
            "env": "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF",
            "kind": "production_origin_trust_anchor_publisher",
            "binds": "production_origin_trust_anchor.publisher_public_key_ref",
        },
    ]


def evidence_schemas() -> dict[str, str]:
    return {
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


def evidence_field_checklist() -> dict[str, Any]:
    return {
        "production_origin_trust_anchor": production_origin_trust_anchor_requirements(),
        "receipt_kms_hsm_custody": {
            "required_top_level_fields": [
                "schema_version",
                "custody_mode",
                "provider",
                "provider_key_ref",
                "key_id",
                "signer_ref",
                "public_key_hex",
                "signing_domain",
                "key_material_exportable",
                "local_seed_material_present",
                "runtime_binding",
                "runtime_signing_probe",
                "operator_attestation",
                "evidence_artifacts",
            ],
            "production_ready_required_top_level_fields": ["production_origin_proof"],
            "closed_shape_fields": sorted(KMS_HSM_CUSTODY_EVIDENCE_FIELDS),
            "nested_closed_shape_fields": {"runtime_binding": sorted(RUNTIME_BINDING_FIELDS), "runtime_signing_probe": sorted(RUNTIME_SIGNING_PROBE_FIELDS), "operator_attestation": sorted(OPERATOR_ATTESTATION_FIELDS)},
            "required_values": {
                "schema_version": KMS_HSM_EVIDENCE_SCHEMA,
                "custody_mode": ["kms", "hsm", "kms_hsm"],
                "signing_domain": SIGNING_DOMAIN,
                "provider_key_ref": "non-local external KMS/HSM provider reference with no raw whitespace",
                "signer_ref": "non-local external runtime signer reference with no raw whitespace",
                "public_key_hex": "64 lowercase hex characters with no surrounding whitespace",
                "key_material_exportable": False,
                "local_seed_material_present": False,
                "runtime_binding.mode": "external_command",
                "runtime_binding.local_seed_fallback": False,
                "runtime_binding.request_schema": REQUEST_SCHEMA,
                "runtime_binding.response_schema": RESPONSE_SCHEMA,
                "runtime_signing_probe.probe_type": RUNTIME_SIGNING_PROBE_TYPE,
                "runtime_signing_probe.request_schema": REQUEST_SCHEMA,
                "runtime_signing_probe.response_schema": RESPONSE_SCHEMA,
                "runtime_signing_probe.signing_domain": SIGNING_DOMAIN,
                "runtime_signing_probe.key_binding": "key_id, public_key_hex, and signer_ref must match the top-level KMS/HSM evidence runtime binding; signer_ref must contain no raw whitespace",
                "runtime_signing_probe.public_key_hex": "64 lowercase hex characters with no surrounding whitespace matching top-level public_key_hex",
                "runtime_signing_probe.canonical_header_hex": "non-empty lowercase hex bytes with no surrounding whitespace signed by the runtime external signer",
                "runtime_signing_probe.request_sha256_hex": "SHA-256 of the canonical external signer request JSON as 64 lowercase hex characters with no surrounding whitespace",
                "runtime_signing_probe.response_sha256_hex": "SHA-256 of the canonical external signer response JSON as 64 lowercase hex characters with no surrounding whitespace",
                "runtime_signing_probe.signature_hex": "128 lowercase hex characters with no surrounding whitespace for the Ed25519 signature over signing_domain || 0x00 || canonical_header_hex bytes, verified with public_key_hex",
                "runtime_signing_probe.signature_sha256_hex": "SHA-256 of the raw runtime probe signature_hex bytes as 64 lowercase hex characters with no surrounding whitespace",
                "runtime_signing_probe.signed_at": (
                    "timezone-aware ISO-8601 timestamp no more than 24 hours "
                    "before validation time and no more than 300 seconds in "
                    "the future"
                ),
                "operator_attestation.issued_at": (
                    "timezone-aware ISO-8601 timestamp; must not be more than "
                    "300 seconds in the future at validation time"
                ),
                "operator_attestation.valid_until": (
                    "timezone-aware ISO-8601 timestamp; must be after "
                    "operator_attestation.issued_at and in the future at "
                    "validation time"
                ),
            },
            "required_controls": sorted(KMS_HSM_REQUIRED_CONTROLS), "controls_closed_set": True, "controls_duplicate_free": True,
            "non_local_reference_fields": sorted(KMS_HSM_NON_LOCAL_REFERENCE_FIELDS),
            "artifact_requirements": {
                "minimum_count": 2,
                "minimum_distinct_uri_count": 2,
                "minimum_distinct_digest_count": 2,
                "required_fields": list(EVIDENCE_ARTIFACT_FIELDS), "closed_shape": "no fields beyond required_fields are accepted",
                "allowed_kinds": sorted(KMS_HSM_EVIDENCE_ARTIFACT_KINDS),
                "uri": "non-local external artifact reference with no raw whitespace",
                "sha256_hex": "64 lowercase hex characters with no surrounding whitespace",
            },
            "production_origin_proof_requirements": production_origin_proof_requirements(),
            "forbidden_secret_fields": sorted(KMS_HSM_FORBIDDEN_SECRET_KEYS),
            "forbidden_secret_field_name_matching": (
                "recursive case-insensitive normalized matching rejects "
                "snake_case, camelCase, kebab-case, and compact aliases of "
                "forbidden secret field names"
            ),
        },
        "compliance_certification": {
            "required_top_level_fields": [
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
            ],
            "production_ready_required_top_level_fields": ["production_origin_proof"],
            "closed_shape_fields": sorted(COMPLIANCE_CERTIFICATION_EVIDENCE_FIELDS),
            "nested_closed_shape_fields": {"external_review": sorted(EXTERNAL_REVIEW_FIELDS), "scope": sorted(COMPLIANCE_SCOPE_FIELDS), "immutability_evidence": sorted(IMMUTABILITY_EVIDENCE_FIELDS)},
            "required_values": {
                "schema_version": COMPLIANCE_EVIDENCE_SCHEMA,
                "framework": sorted(SUPPORTED_FRAMEWORKS),
                "string_values": "required string fields must not include surrounding whitespace",
                "report_ref": "non-local external compliance report reference with no raw whitespace",
                "external_review.assurance_level": ["limited", "reasonable", "certified"],
                "external_review.issued_at": (
                    "timezone-aware ISO-8601 timestamp; must not be more than "
                    "300 seconds in the future at validation time"
                ),
                "external_review.valid_until": (
                    "timezone-aware ISO-8601 timestamp; must be after "
                    "external_review.issued_at and in the future at validation "
                    "time"
                ),
                "external_review.nda_required": True,
                "scope.product": "CortexDB",
                "scope.receipt_schema": "accountability_receipt.v1",
                "scope.production_grade_public_receipts": True,
                "immutability_evidence.external_immutable_store": True,
                "immutability_evidence.append_only_export": True,
                "immutability_evidence.tamper_evident_reviewed": True,
                "immutability_evidence.retention_days": "integer >= 365",
                "immutability_evidence.retention_policy_ref": "non-local external retention policy reference with no raw whitespace",
                "immutability_evidence.tamper_evidence_ref": "non-local external tamper evidence reference with no raw whitespace",
            },
            "required_controls": sorted(COMPLIANCE_REQUIRED_CONTROLS), "controls_closed_set": True, "controls_duplicate_free": True,
            "non_local_reference_fields": sorted(COMPLIANCE_NON_LOCAL_REFERENCE_FIELDS),
            "required_operator_responsibilities": sorted(COMPLIANCE_REQUIRED_OPERATOR_RESPONSIBILITIES), "operator_responsibilities_closed_set": True, "operator_responsibilities_duplicate_free": True,
            "artifact_requirements": {
                "minimum_count": 2,
                "minimum_distinct_uri_count": 2,
                "minimum_distinct_digest_count": 2,
                "required_fields": list(EVIDENCE_ARTIFACT_FIELDS), "closed_shape": "no fields beyond required_fields are accepted",
                "allowed_kinds": sorted(COMPLIANCE_EVIDENCE_ARTIFACT_KINDS),
                "uri": "non-local external artifact reference with no raw whitespace",
                "sha256_hex": "64 lowercase hex characters with no surrounding whitespace",
            },
            "production_origin_proof_requirements": production_origin_proof_requirements(),
            "forbidden_secret_fields": sorted(COMPLIANCE_FORBIDDEN_SECRET_KEYS),
            "forbidden_secret_field_name_matching": (
                "recursive case-insensitive normalized matching rejects "
                "snake_case, camelCase, kebab-case, and compact aliases of "
                "forbidden secret field names"
            ),
        },
    }
