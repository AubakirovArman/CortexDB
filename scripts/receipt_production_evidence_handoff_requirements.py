"""Requirement blocks for the production evidence handoff payload."""

from __future__ import annotations

from typing import Any

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

def production_origin_trust_anchor_requirements() -> dict[str, Any]:
    return {
        "schema_version": TRUST_ANCHOR_EVIDENCE_SCHEMA,
        "required_top_level_fields": sorted(TRUST_ANCHOR_EVIDENCE_FIELDS),
        "closed_shape_fields": sorted(TRUST_ANCHOR_EVIDENCE_FIELDS),
        "non_local_reference_fields": sorted(TRUST_ANCHOR_NON_LOCAL_REFERENCE_FIELDS),
        "required_values": {
            "trust_anchor_type": TRUST_ANCHOR_TYPE,
            "external_control_plane": True,
            "reference_values": (
                "top-level trust-anchor reference fields must be non-local "
                "external references with no raw whitespace"
            ),
            "key_ids": "key_attestor_key_id and publisher_key_id must contain no whitespace",
            "key_attestor_public_key_hex": "64 lowercase hex characters with no surrounding whitespace",
            "key_attestor_binding": (
                "key_attestor_key_id, key_attestor_public_key_hex, "
                "key_attestor_ref, and key_attestor_public_key_ref must match "
                "the separately supplied RECEIPT_PRODUCTION_ORIGIN_EXPECTED_* "
                "strict preflight inputs and both production_origin_proof "
                "objects"
            ),
            "publisher_binding": (
                "publisher_key_id, publisher_public_key_hex, publisher_ref, "
                "and publisher_public_key_ref must match the separately "
                "supplied RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_* "
                "strict preflight inputs and must be distinct from the "
                "key_attestor_* identity fields"
            ),
            "publisher_public_key_hex": "64 lowercase hex characters with no surrounding whitespace",
            "signature_algorithm": "ed25519",
            "signature_hex": (
                "128 lowercase hex characters with no surrounding whitespace; Ed25519 signature over the "
                "trust-anchor publication JSON after removing signature_hex "
                "and signature_sha256_hex with "
                f"{TRUST_ANCHOR_SIGNING_DOMAIN} domain bytes"
            ),
            "signature_sha256_hex": "SHA-256 of the raw signature_hex bytes as 64 lowercase hex characters with no surrounding whitespace",
            "published_at": (
                "timezone-aware ISO-8601 timestamp; must not be more than 300 "
                "seconds in the future at validation time"
            ),
            "valid_until": (
                "timezone-aware ISO-8601 timestamp; must be after published_at "
                "and in the future at validation time"
            ),
        },
        "signing_domain": TRUST_ANCHOR_SIGNING_DOMAIN,
        "required_controls": sorted(TRUST_ANCHOR_REQUIRED_CONTROLS),
        "controls_closed_set": True,
        "controls_duplicate_free": True,
        "artifact_requirements": {
            "minimum_count": 2,
            "minimum_distinct_uri_count": 2,
            "minimum_distinct_digest_count": 2,
            "required_fields": list(EVIDENCE_ARTIFACT_FIELDS),
            "closed_shape": "no fields beyond required_fields are accepted",
            "allowed_kinds": sorted(TRUST_ANCHOR_EVIDENCE_ARTIFACT_KINDS),
            "uri": "non-local external artifact reference with no raw whitespace",
            "sha256_hex": "64 lowercase hex characters with no surrounding whitespace",
        },
        "forbidden_secret_fields": sorted(TRUST_ANCHOR_FORBIDDEN_SECRET_KEYS),
        "forbidden_secret_field_name_matching": (
            "recursive case-insensitive normalized matching rejects snake_case, "
            "camelCase, kebab-case, and compact aliases of forbidden secret "
            "field names"
        ),
    }


def production_origin_proof_requirements() -> dict[str, Any]:
    return {
        "schema_version": PRODUCTION_ORIGIN_PROOF_SCHEMA,
        "required_fields": [
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
        ],
        "required_values": {
            "external_control_plane": True,
            "string_values": (
                "required production_origin_proof string fields must not include "
                "surrounding whitespace"
            ),
            "key_ids": "issuer_key_id and key_attestor_key_id must contain no whitespace",
            "reviewer_identity": "reviewed_by must contain no whitespace",
            "proof_sha256_hex": (
                "SHA-256 of production_origin_proof after removing "
                "proof_sha256_hex, serialized with sorted keys and compact "
                "separators"
            ),
            "signed_statement_sha256_hex": (
                "SHA-256 of signed_statement, serialized with sorted keys and "
                "compact separators"
            ),
            "evidence_sha256_hex": (
                "SHA-256 of the evidence JSON object after removing "
                "production_origin_proof, serialized with sorted keys and "
                "compact separators"
            ),
            "issuer_public_key_hex": "64 lowercase hex characters",
            "issuer_key_attestation_sha256_hex": (
                "SHA-256 of issuer_key_attestation, serialized with sorted keys "
                "and compact separators"
            ),
            "key_attestor_public_key_hex": "64 lowercase hex characters",
            "key_attestor_trust_anchor_binding": (
                "key_attestor_key_id, key_attestor_public_key_hex, "
                "key_attestor_ref, and key_attestor_public_key_ref must match "
                "the separately supplied canonical lowercase "
                "RECEIPT_PRODUCTION_ORIGIN_EXPECTED_* strict preflight inputs"
            ),
            "issuer_attestor_independence": (
                "issuer_ref, issuer_key_id, issuer_public_key_ref, and "
                "issuer_public_key_hex must be distinct from the corresponding "
                "key_attestor_ref, key_attestor_key_id, "
                "key_attestor_public_key_ref, and key_attestor_public_key_hex"
            ),
            "reviewer_independence": (
                "reviewed_by must be distinct from issuer_ref, issuer_key_id, "
                "issuer_public_key_ref, key_attestor_ref, key_attestor_key_id, "
                "and key_attestor_public_key_ref"
            ),
            "key_attestation_signature_algorithm": "ed25519",
            "key_attestation_signature_hex": (
                "128 lowercase hex characters; Ed25519 signature over the "
                "issuer_key_attestation canonical JSON with "
                f"{PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN} domain bytes"
            ),
            "key_attestation_signature_sha256_hex": (
                "SHA-256 of the raw key_attestation_signature_hex bytes"
            ),
            "signature_algorithm": "ed25519",
            "signature_hex": (
                "128 lowercase hex characters; Ed25519 signature over the "
                "signed_statement canonical JSON with "
                f"{PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN} domain bytes"
            ),
            "signature_sha256_hex": "SHA-256 of the raw signature_hex bytes",
            "issued_at": (
                "timezone-aware ISO-8601 timestamp; must not be more than 300 "
                "seconds in the future at validation time"
            ),
            "expires_at": (
                "timezone-aware ISO-8601 timestamp; must be after issued_at "
                "and in the future at validation time"
            ),
        },
        "issuer_key_attestation_requirements": production_origin_key_attestation_requirements(),
        "signed_statement_requirements": production_origin_statement_requirements(),
        "closed_shape": "no fields beyond required_fields are accepted",
        "reference_boundary": (
            "proof_ref, issuer_ref, issuer_public_key_ref, "
            "issuer_key_attestation_ref, key_attestor_ref, "
            "key_attestor_public_key_ref, key_attestation_signature_ref, "
            "signed_statement_ref, and signature_ref must point to non-local "
            "external operator evidence; local paths, generated artifacts, "
            "fixtures, temporary paths, loopback URLs, file: refs, and raw "
            "whitespace are rejected"
        ),
    }


def production_origin_key_attestation_requirements() -> dict[str, Any]:
    return {
        "schema_version": PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
        "attestation_type": PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
        "required_fields": [
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
        ],
        "required_values": {
            "proof_schema_version": PRODUCTION_ORIGIN_PROOF_SCHEMA,
            "attestation_type": PRODUCTION_ORIGIN_KEY_ATTESTATION_TYPE,
            "statement_signing_domain": PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
            "external_control_plane": True,
            "key_attestation_signature_algorithm": "ed25519",
        },
        "signing_domain": PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
        "closed_shape": "no fields beyond required_fields are accepted",
        "binding_rule": (
            "issuer key fields must match the enclosing production_origin_proof; "
            "issuer_key_attestation_sha256_hex, key_attestation_signature_hex, "
            "and key_attestation_signature_sha256_hex must remain outside "
            "issuer_key_attestation to avoid signing self-referential digest "
            "or signature fields"
        ),
    }


def production_origin_statement_requirements() -> dict[str, Any]:
    return {
        "schema_version": PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
        "statement_type": PRODUCTION_ORIGIN_STATEMENT_TYPE,
        "required_fields": [
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
        ],
        "required_values": {
            "proof_schema_version": PRODUCTION_ORIGIN_PROOF_SCHEMA,
            "external_control_plane": True,
            "statement_type": PRODUCTION_ORIGIN_STATEMENT_TYPE,
            "signature_algorithm": "ed25519",
        },
        "signing_domain": PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
        "closed_shape": "no fields beyond required_fields are accepted",
        "binding_rule": (
            "every overlapping signed_statement field must match the enclosing "
            "production_origin_proof and evidence_schema_version/evidence_sha256_hex "
            "must match the evidence body; signed_statement_sha256_hex, "
            "signature_sha256_hex, and signature_hex must remain outside "
            "signed_statement to avoid signing self-referential digest or "
            "signature fields"
        ),
    }


def origin_boundary() -> dict[str, Any]:
    return {
        "accepted_origin": "operator",
        "rejected_origins": [
            "generated_local_artifact",
            "temporary_local_artifact",
            "local_reference_artifact",
            "synthetic_fixture",
            "missing",
            "unknown",
        ],
        "rejected_reference_families": [
            "fixtures/",
            "target/",
            "file:",
            "local filesystem paths",
            "temporary directories",
            "loopback URLs",
            "local transports",
            "shell or user-local expansions",
        ],
    }
