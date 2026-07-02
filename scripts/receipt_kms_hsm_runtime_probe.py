"""Validate a KMS/HSM custody runtime signing probe."""

from __future__ import annotations

import hashlib
import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from evidence_origin import canonical_json_sha256_hex


SIGNING_DOMAIN = "cortexdb.accountability_receipt.sign.v1"
REQUEST_SCHEMA = "cortexdb.receipt_external_sign_request.v1"
RESPONSE_SCHEMA = "cortexdb.receipt_external_signature.v1"
RUNTIME_SIGNING_PROBE_TYPE = "receipt_external_signer_live_probe"
RUNTIME_SIGNING_PROBE_MAX_AGE_SECONDS = 24 * 60 * 60
RUNTIME_SIGNING_PROBE_MAX_FUTURE_SKEW_SECONDS = 5 * 60
RUNTIME_SIGNING_PROBE_FIELDS = {"probe_type", "request_schema", "response_schema", "key_id", "public_key_hex", "signer_ref", "signing_domain", "canonical_header_hex", "request_sha256_hex", "response_sha256_hex", "signature_hex", "signature_sha256_hex", "signed_at"}


def validate_runtime_signing_probe(
    raw: Any,
    *,
    key_id: str,
    public_key_hex: str,
    signer_ref: str,
    failures: list[str],
    require_fresh_signed_at: bool = True,
) -> dict[str, Any]:
    if not isinstance(raw, dict):
        failures.append("runtime_signing_probe must be an object")
        return {"provided": False}
    for field in sorted(set(raw) - RUNTIME_SIGNING_PROBE_FIELDS):
        failures.append(f"runtime_signing_probe.{field} is not allowed")

    probe_type = string_field(raw, "probe_type", failures)
    request_schema = string_field(raw, "request_schema", failures)
    response_schema = string_field(raw, "response_schema", failures)
    probe_key_id = string_field(raw, "key_id", failures)
    probe_public_key_hex = string_field(raw, "public_key_hex", failures)
    probe_signer_ref = string_field(raw, "signer_ref", failures)
    signing_domain = string_field(raw, "signing_domain", failures)
    canonical_header_hex = string_field(raw, "canonical_header_hex", failures)
    request_sha256_hex = string_field(raw, "request_sha256_hex", failures)
    response_sha256_hex = string_field(raw, "response_sha256_hex", failures)
    signature_hex = string_field(raw, "signature_hex", failures)
    signature_sha256_hex = string_field(raw, "signature_sha256_hex", failures)
    signed_at = string_field(raw, "signed_at", failures)

    validate_probe_values(
        failures,
        probe_type=probe_type,
        request_schema=request_schema,
        response_schema=response_schema,
        probe_key_id=probe_key_id,
        probe_public_key_hex=probe_public_key_hex,
        probe_signer_ref=probe_signer_ref,
        signing_domain=signing_domain,
        canonical_header_hex=canonical_header_hex,
        request_sha256_hex=request_sha256_hex,
        response_sha256_hex=response_sha256_hex,
        signature_hex=signature_hex,
        signature_sha256_hex=signature_sha256_hex,
        signed_at=signed_at,
        require_fresh_signed_at=require_fresh_signed_at,
        key_id=key_id,
        public_key_hex=public_key_hex,
        signer_ref=signer_ref,
    )

    if is_hex(public_key_hex, 64) and runtime_signing_probe_signature_fields_are_well_formed(raw):
        verify_runtime_signing_probe_signature(
            canonical_header_hex=canonical_header_hex,
            public_key_hex=public_key_hex,
            signature_hex=signature_hex,
            failures=failures,
        )

    return {
        "provided": True,
        "probe_type": probe_type,
        "request_schema": request_schema,
        "response_schema": response_schema,
        "key_id": probe_key_id,
        "public_key_hex": probe_public_key_hex,
        "signer_ref": probe_signer_ref,
        "signing_domain": signing_domain,
        "canonical_header_sha256_hex": canonical_header_sha256_hex(canonical_header_hex),
        "request_sha256_hex": request_sha256_hex,
        "response_sha256_hex": response_sha256_hex,
        "signature_sha256_hex": signature_sha256_hex,
        "signed_at": signed_at,
    }


def validate_probe_values(
    failures: list[str],
    *,
    probe_type: str,
    request_schema: str,
    response_schema: str,
    probe_key_id: str,
    probe_public_key_hex: str,
    probe_signer_ref: str,
    signing_domain: str,
    canonical_header_hex: str,
    request_sha256_hex: str,
    response_sha256_hex: str,
    signature_hex: str,
    signature_sha256_hex: str,
    signed_at: str,
    require_fresh_signed_at: bool,
    key_id: str,
    public_key_hex: str,
    signer_ref: str,
) -> None:
    if probe_type != RUNTIME_SIGNING_PROBE_TYPE:
        failures.append(f"runtime_signing_probe.probe_type must be {RUNTIME_SIGNING_PROBE_TYPE}")
    if request_schema != REQUEST_SCHEMA:
        failures.append(f"runtime_signing_probe.request_schema must be {REQUEST_SCHEMA}")
    if response_schema != RESPONSE_SCHEMA:
        failures.append(f"runtime_signing_probe.response_schema must be {RESPONSE_SCHEMA}")
    if probe_key_id != key_id:
        failures.append("runtime_signing_probe.key_id must match key_id")
    if not is_hex(probe_public_key_hex, 64):
        failures.append("runtime_signing_probe.public_key_hex must be 64 lowercase hex characters")
    if probe_public_key_hex != public_key_hex:
        failures.append("runtime_signing_probe.public_key_hex must match public_key_hex")
    if probe_signer_ref != signer_ref:
        failures.append("runtime_signing_probe.signer_ref must match signer_ref")
    if any(character.isspace() for character in probe_signer_ref):
        failures.append("runtime_signing_probe.signer_ref must contain no whitespace")
    if signing_domain != SIGNING_DOMAIN:
        failures.append(f"runtime_signing_probe.signing_domain must be {SIGNING_DOMAIN}")
    if not is_even_hex(canonical_header_hex):
        failures.append("runtime_signing_probe.canonical_header_hex must be non-empty lowercase hex bytes")
    if not is_hex(request_sha256_hex, 64):
        failures.append("runtime_signing_probe.request_sha256_hex must be 64 lowercase hex characters")
    if not is_hex(response_sha256_hex, 64):
        failures.append("runtime_signing_probe.response_sha256_hex must be 64 lowercase hex characters")
    if not is_hex(signature_hex, 128):
        failures.append("runtime_signing_probe.signature_hex must be 128 lowercase hex characters")
    if not is_hex(signature_sha256_hex, 64):
        failures.append("runtime_signing_probe.signature_sha256_hex must be 64 lowercase hex characters")
    signed_at_time = parse_probe_timestamp(signed_at)
    if signed_at_time is None:
        failures.append("runtime_signing_probe.signed_at must be timezone-aware ISO-8601")
    elif require_fresh_signed_at:
        validate_probe_freshness(signed_at_time, failures)
    validate_probe_digests(
        failures,
        canonical_header_hex=canonical_header_hex,
        request_sha256_hex=request_sha256_hex,
        response_sha256_hex=response_sha256_hex,
        signature_hex=signature_hex,
        signature_sha256_hex=signature_sha256_hex,
        key_id=key_id,
        public_key_hex=public_key_hex,
        signer_ref=signer_ref,
    )


def validate_probe_digests(
    failures: list[str],
    *,
    canonical_header_hex: str,
    request_sha256_hex: str,
    response_sha256_hex: str,
    signature_hex: str,
    signature_sha256_hex: str,
    key_id: str,
    public_key_hex: str,
    signer_ref: str,
) -> None:
    expected_request = {
        "schema_version": REQUEST_SCHEMA,
        "key_id": key_id,
        "public_key_hex": public_key_hex,
        "signing_domain": SIGNING_DOMAIN,
        "signer_ref": signer_ref,
        "canonical_header_hex": canonical_header_hex,
    }
    if is_hex(request_sha256_hex, 64) and request_sha256_hex != canonical_json_sha256_hex(expected_request):
        failures.append("runtime_signing_probe.request_sha256_hex must match external signer request")
    expected_response = {
        "schema_version": RESPONSE_SCHEMA,
        "key_id": key_id,
        "public_key_hex": public_key_hex,
        "signature_hex": signature_hex,
    }
    if is_hex(response_sha256_hex, 64) and response_sha256_hex != canonical_json_sha256_hex(expected_response):
        failures.append("runtime_signing_probe.response_sha256_hex must match external signer response")
    if is_hex(signature_hex, 128) and is_hex(signature_sha256_hex, 64):
        if signature_sha256_hex != hashlib.sha256(bytes.fromhex(signature_hex)).hexdigest():
            failures.append("runtime_signing_probe.signature_sha256_hex must match signature_hex bytes")


def runtime_signing_probe_signature_fields_are_well_formed(raw: dict[str, Any]) -> bool:
    canonical_header_hex = raw.get("canonical_header_hex")
    public_key_hex = raw.get("public_key_hex")
    signature_hex = raw.get("signature_hex")
    return (
        isinstance(canonical_header_hex, str)
        and is_even_hex(canonical_header_hex)
        and isinstance(public_key_hex, str)
        and is_hex(public_key_hex, 64)
        and isinstance(signature_hex, str)
        and is_hex(signature_hex, 128)
    )


def verify_runtime_signing_probe_signature(
    *,
    canonical_header_hex: str,
    public_key_hex: str,
    signature_hex: str,
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
        public_key_hex,
        "--signature-hex",
        signature_hex,
    ]
    result = subprocess.run(
        command,
        cwd=Path(__file__).resolve().parents[1],
        input=SIGNING_DOMAIN.encode("utf-8") + b"\0" + bytes.fromhex(canonical_header_hex),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        failures.append(
            "runtime_signing_probe.signature_hex must verify canonical_header_hex with public_key_hex"
        )


def string_field(value: dict[str, Any], name: str, failures: list[str]) -> str:
    raw = value.get(name)
    if isinstance(raw, str) and raw.strip():
        if raw != raw.strip():
            failures.append(f"runtime_signing_probe.{name} must not include surrounding whitespace")
        return raw.strip()
    failures.append(f"runtime_signing_probe.{name} must be a non-empty string")
    return ""


def is_hex(value: str, length: int) -> bool:
    return len(value) == length and all(character in "0123456789abcdef" for character in value)


def is_even_hex(value: str) -> bool:
    return bool(value) and len(value) % 2 == 0 and all(
        character in "0123456789abcdef" for character in value
    )


def canonical_header_sha256_hex(canonical_header_hex: str) -> str | None:
    if not is_even_hex(canonical_header_hex):
        return None
    return hashlib.sha256(bytes.fromhex(canonical_header_hex)).hexdigest()


def parse_probe_timestamp(value: str) -> datetime | None:
    if not value:
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        return None
    return normalize_probe_timestamp(parsed)


def normalize_probe_timestamp(value: datetime) -> datetime:
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def validate_probe_freshness(
    signed_at: datetime,
    failures: list[str],
    *,
    now: datetime | None = None,
) -> None:
    current = normalize_probe_timestamp(now or datetime.now(timezone.utc))
    if signed_at > current + timedelta(seconds=RUNTIME_SIGNING_PROBE_MAX_FUTURE_SKEW_SECONDS):
        failures.append("runtime_signing_probe.signed_at must not be more than 300 seconds in the future")
    if current - signed_at > timedelta(seconds=RUNTIME_SIGNING_PROBE_MAX_AGE_SECONDS):
        failures.append("runtime_signing_probe.signed_at must be within 24 hours of validation time")
