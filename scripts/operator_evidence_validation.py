"""Shared helpers for operator evidence validators."""

from __future__ import annotations

from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

try:
    from evidence_origin_references import is_local_reference
except ModuleNotFoundError:  # Allows `from scripts.operator_evidence_validation import ...`.
    from scripts.evidence_origin_references import is_local_reference


EVIDENCE_ARTIFACT_FIELDS = ("kind", "uri", "sha256_hex")
EVIDENCE_ARTIFACT_FIELD_SET = set(EVIDENCE_ARTIFACT_FIELDS)
FORBIDDEN_SECRET_KEYS = frozenset(
    {
        "api_key",
        "password",
        "passphrase",
        "private_key",
        "secret",
        "seed_hex",
        "signing_seed_hex",
        "token",
    }
)
FORBIDDEN_SECRET_KEY_ALIASES = frozenset(
    {
        "accesskey",
        "accesstoken",
        "apikey",
        "apitoken",
        "clientsecret",
        "password",
        "passphrase",
        "privatekey",
        "secret",
        "secretkey",
        "seedhex",
        "signingseed",
        "signingseedhex",
        "token",
    }
)


def invalid_result(path: Path, failures: list[str]) -> dict[str, Any]:
    return {
        "provided": True,
        "path": str(path),
        "valid": False,
        "summary": {},
        "evidence_origin": "unknown",
        "synthetic_evidence": False,
        "synthetic_evidence_reasons": [],
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


def is_hex(value: str, length: int) -> bool:
    return len(value) == length and all(character in "0123456789abcdef" for character in value)


def parse_timestamp(value: str, label: str, failures: list[str]) -> datetime | None:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        failures.append(f"{label} must be timezone-aware ISO-8601")
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        failures.append(f"{label} must be timezone-aware ISO-8601")
        return None
    return normalize_timestamp(parsed)


def normalize_timestamp(value: datetime) -> datetime:
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def validate_not_expired(
    timestamp: datetime | None,
    failures: list[str],
    *,
    label: str,
    now: datetime | None = None,
) -> None:
    if timestamp is None:
        return
    current = normalize_timestamp(now or datetime.now(timezone.utc))
    if timestamp <= current:
        failures.append(f"{label} must be in the future")


def validate_not_future(
    timestamp: datetime | None,
    failures: list[str],
    *,
    label: str,
    now: datetime | None = None,
    max_future_skew_seconds: int = 300,
) -> None:
    if timestamp is None:
        return
    current = normalize_timestamp(now or datetime.now(timezone.utc))
    if timestamp > current + timedelta(seconds=max_future_skew_seconds):
        failures.append(
            f"{label} must not be more than {max_future_skew_seconds} seconds in the future"
        )


def forbidden_secret_paths(value: Any, prefix: str = "") -> list[str]:
    paths: list[str] = []
    if isinstance(value, dict):
        for key, nested in value.items():
            field_path = f"{prefix}.{key}" if prefix else str(key)
            if is_forbidden_secret_key(str(key)):
                paths.append(field_path)
            paths.extend(forbidden_secret_paths(nested, field_path))
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            paths.extend(forbidden_secret_paths(nested, f"{prefix}[{index}]"))
    return paths


def is_forbidden_secret_key(key: str) -> bool:
    lowered = key.lower()
    normalized = "".join(character for character in lowered if character.isalnum())
    return lowered in FORBIDDEN_SECRET_KEYS or normalized in FORBIDDEN_SECRET_KEY_ALIASES


def validate_required_controls(
    raw: Any,
    failures: list[str],
    *,
    field_name: str,
    required_controls: set[str],
) -> None:
    if not isinstance(raw, list) or not all(isinstance(item, str) for item in raw):
        failures.append(f"{field_name} must be a string list")
        return

    supplied = set(raw)
    missing = sorted(required_controls.difference(supplied))
    if missing:
        failures.append(f"{field_name} missing required controls: {', '.join(missing)}")
    unsupported = sorted(supplied.difference(required_controls))
    if unsupported:
        failures.append(f"{field_name} contains unsupported controls: {', '.join(unsupported)}")
    duplicates = sorted({item for item in raw if raw.count(item) > 1})
    if duplicates:
        failures.append(f"{field_name} contains duplicate controls: {', '.join(duplicates)}")


def validate_evidence_artifacts(
    raw: Any,
    failures: list[str],
    *,
    field_name: str = "evidence_artifacts",
    minimum_count: int = 2,
    allowed_kinds: set[str] | None = None,
) -> None:
    if not isinstance(raw, list) or len(raw) < minimum_count:
        failures.append(f"{field_name} must contain at least {minimum_count} artifacts")
        return

    uris: list[str] = []
    digests: list[str] = []
    entries: set[tuple[str, str, str]] = set()
    for index, artifact in enumerate(raw):
        if not isinstance(artifact, dict):
            failures.append(f"{field_name}[{index}] must be an object")
            continue
        for field in sorted(set(artifact) - EVIDENCE_ARTIFACT_FIELD_SET):
            failures.append(f"{field_name}[{index}].{field} is not allowed")
        kind = artifact_string_field(artifact, "kind", field_name, index, failures)
        uri = artifact_string_field(artifact, "uri", field_name, index, failures)
        digest = artifact_string_field(artifact, "sha256_hex", field_name, index, failures)
        if allowed_kinds is not None and kind and kind not in allowed_kinds:
            failures.append(
                f"{field_name}[{index}].kind must be one of {', '.join(sorted(allowed_kinds))}"
            )
        if digest and not is_hex(digest, 64):
            failures.append(f"{field_name}[{index}].sha256_hex must be 64 lowercase hex characters")
        validate_non_local_reference(uri, failures, field_name=f"{field_name}[{index}].uri")
        if any(character.isspace() for character in uri):
            failures.append(f"{field_name}[{index}].uri must contain no raw whitespace")
        if uri:
            uris.append(uri)
        if digest:
            digests.append(digest)
        if kind and uri and digest:
            entry = (kind, uri, digest)
            if entry in entries:
                failures.append(f"{field_name}[{index}] duplicates an earlier artifact entry")
            entries.add(entry)

    if len(set(uris)) < minimum_count:
        failures.append(f"{field_name} must contain at least {minimum_count} distinct artifact URIs")
    if len(set(digests)) < minimum_count:
        failures.append(f"{field_name} must contain at least {minimum_count} distinct artifact digests")


def artifact_string_field(
    value: dict[str, Any],
    name: str,
    field_name: str,
    index: int,
    failures: list[str],
) -> str:
    raw = value.get(name)
    if isinstance(raw, str) and raw.strip():
        if raw != raw.strip():
            failures.append(f"{field_name}[{index}].{name} must not include surrounding whitespace")
        return raw.strip()
    failures.append(f"{field_name}[{index}].{name} must be a non-empty string")
    return ""


def validate_non_local_reference(value: str, failures: list[str], *, field_name: str) -> None:
    if value and is_local_reference(value):
        failures.append(f"{field_name} must not be a local/generated evidence reference")
