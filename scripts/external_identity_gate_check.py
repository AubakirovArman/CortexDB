#!/usr/bin/env python3
"""Validate local external-identity future-epic evidence gates."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


FORBIDDEN_ENDPOINTS = ("/v1/oidc", "/v1/saml", "/v1/identity/callback", "/v1/login")

GATES: dict[str, dict[str, object]] = {
    "oidc-contract": {
        "schema": "cortexdb.external_identity.oidc_contract_gate.v1",
        "markers": [
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "OIDC Contract Boundary"),
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "Issuer And Audience"),
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "Fail-closed Behavior"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make oidc-auth-contract-check"),
            ("Makefile", "oidc-auth-contract-check"),
        ],
    },
    "policy-mapping": {
        "schema": "cortexdb.external_identity.policy_mapping_gate.v1",
        "markers": [
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "Role And Scope Mapping"),
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "Mapping Fixture"),
            ("docs/archive/EXTERNAL_IDENTITY_ADMIN_RUNBOOK.md", "Static Token Migration"),
            ("docs/AUTH.md", "AgentView binding"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make identity-policy-mapping-check"),
            ("Makefile", "identity-policy-mapping-check"),
        ],
        "fixture": "crates/cortex-server/fixtures/external_identity_policy_mapping_v1.json",
    },
    "rotation": {
        "schema": "cortexdb.external_identity.rotation_gate.v1",
        "markers": [
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "JWKS And Rotation"),
            ("docs/EXTERNAL_IDENTITY_DESIGN.md", "Rotation Fixture"),
            ("docs/archive/EXTERNAL_IDENTITY_ADMIN_RUNBOOK.md", "Rotation Procedure"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make auth-rotation-check"),
            ("Makefile", "auth-rotation-check"),
        ],
        "fixture": "crates/cortex-server/fixtures/external_identity_rotation_v1.json",
    },
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read fixture {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"fixture {path} is invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"fixture {path} must be a JSON object")
    return value


def validate_markers(markers: list[tuple[str, str]]) -> list[str]:
    failures: list[str] = []
    for file_name, marker in markers:
        if marker not in read(Path(file_name)):
            failures.append(f"marker {marker!r} missing from {file_name}")
    return failures


def validate_no_forbidden_endpoints() -> list[str]:
    failures: list[str] = []
    for relative in [
        Path("docs/openapi.yaml"),
        Path("crates/cortex-server/src/router.rs"),
        Path("crates/cortex-server/src/auth.rs"),
    ]:
        text = read(relative)
        for endpoint in FORBIDDEN_ENDPOINTS:
            if endpoint in text:
                failures.append(f"{relative}: forbidden external-identity endpoint {endpoint!r} is exposed")
    return failures


def validate_mapping_fixture(path: Path) -> list[str]:
    failures: list[str] = []
    value = load_json(path)
    if value.get("schema_version") != "cortexdb.external_identity.policy_mapping.v1":
        failures.append("mapping fixture has wrong schema_version")
    if value.get("protocol") != "oidc":
        failures.append("mapping fixture must target oidc first")
    if value.get("trust_provider_groups_as_scopes") is not False:
        failures.append("provider groups must not be trusted directly as CortexDB scopes")
    if value.get("fail_closed_on_missing_mapping") is not True:
        failures.append("missing identity mapping must fail closed")
    if value.get("static_tokens_supported") is not True:
        failures.append("static token compatibility must remain true")
    mappings = value.get("mappings")
    if not isinstance(mappings, list) or not mappings:
        return failures + ["mapping fixture must contain at least one mapping"]
    for index, mapping in enumerate(mappings):
        if not isinstance(mapping, dict):
            failures.append(f"mapping {index} must be an object")
            continue
        for field in ["external_group", "role", "tenant", "scopes", "agent_id"]:
            if field not in mapping:
                failures.append(f"mapping {index} missing {field}")
        if mapping.get("role") not in {"admin", "data"}:
            failures.append(f"mapping {index} has unsupported role")
        scopes = mapping.get("scopes")
        if not isinstance(scopes, list) or not all(isinstance(scope, str) and scope for scope in scopes):
            failures.append(f"mapping {index} scopes must be non-empty strings")
    return failures


def validate_local_claim_verifier() -> list[str]:
    failures: list[str] = []
    source = read(Path("crates/cortex-server/src/external_identity.rs"))
    markers = [
        "verify_oidc_claims",
        "InvalidIssuer",
        "InvalidAudience",
        "ExpiredToken",
        "TokenNotYetValid",
        "MissingMapping",
        "InvalidMapping",
        "InvalidConfig",
        "validate_external_identity_config",
        "project:investments",
    ]
    for marker in markers:
        if marker not in source:
            failures.append(f"external_identity.rs missing verifier marker {marker!r}")
    return failures


def validate_rotation_fixture(path: Path) -> list[str]:
    failures: list[str] = []
    value = load_json(path)
    if value.get("schema_version") != "cortexdb.external_identity.rotation.v1":
        failures.append("rotation fixture has wrong schema_version")
    if value.get("jwks_cache_ttl_seconds", 0) <= 0:
        failures.append("jwks_cache_ttl_seconds must be positive")
    if not isinstance(value.get("audience"), str) or not value["audience"]:
        failures.append("audience must be present")
    jwks_url = value.get("jwks_url")
    if not isinstance(jwks_url, str) or not jwks_url.startswith("https://"):
        failures.append("jwks_url must be https")
    algorithms = value.get("allowed_algorithms")
    if not isinstance(algorithms, list) or not algorithms:
        failures.append("allowed_algorithms must be non-empty")
    elif any(algorithm not in {"RS256", "ES256", "PS256"} for algorithm in algorithms):
        failures.append("allowed_algorithms must use asymmetric production algorithms")
    if value.get("request_timeout_ms", 0) <= 0:
        failures.append("request_timeout_ms must be positive")
    if value.get("fail_open") is not False:
        failures.append("fail_open must be false")
    if value.get("unknown_kid_policy") != "deny":
        failures.append("unknown kid policy must deny")
    if value.get("provider_outage_policy") != "fail_closed_for_new_tokens":
        failures.append("provider outage policy must fail closed for new tokens")
    rejected = value.get("rejected_cases")
    required = {"invalid_issuer", "invalid_audience", "expired_token", "unknown_kid", "missing_mapping"}
    if not isinstance(rejected, list) or required.difference(rejected):
        failures.append("rotation fixture missing required rejected cases")
    if value.get("audit_principal_without_token") is not True:
        failures.append("audit must identify principal without logging token")
    return failures


def validate_provider_config_marker() -> list[str]:
    source = read(Path("crates/cortex-server/src/external_identity/provider.rs"))
    markers = [
        "OidcProviderConfig",
        "validate_oidc_provider_config",
        "InvalidJwksUrl",
        "FailOpenNotAllowed",
        '"RS256" | "ES256" | "PS256"',
    ]
    return [
        f"external_identity/provider.rs missing provider marker {marker!r}"
        for marker in markers
        if marker not in source
    ]


def validate_audit_contract_marker() -> list[str]:
    source = read(Path("crates/cortex-server/src/external_identity/audit.rs"))
    markers = [
        "ExternalIdentityAuditRecord",
        "external_identity_decision_audit_record",
        "external_identity_failure_audit_record",
        "token_logged: false",
        "claims_logged: false",
    ]
    return [
        f"external_identity/audit.rs missing audit marker {marker!r}"
        for marker in markers
        if marker not in source
    ]


def validate(gate: str) -> dict[str, Any]:
    spec = GATES[gate]
    failures = validate_markers(spec["markers"])  # type: ignore[arg-type]
    checks = {
        "markers": not failures,
        "no_external_identity_routes": True,
        "mapping_fixture": True,
        "rotation_fixture": True,
        "local_claim_verifier": True,
    }

    if gate == "oidc-contract":
        endpoint_failures = validate_no_forbidden_endpoints()
        failures.extend(endpoint_failures)
        checks["no_external_identity_routes"] = not endpoint_failures
    elif gate == "policy-mapping":
        mapping_failures = validate_mapping_fixture(Path(spec["fixture"]))  # type: ignore[arg-type]
        mapping_failures.extend(validate_local_claim_verifier())
        mapping_failures.extend(validate_audit_contract_marker())
        failures.extend(mapping_failures)
        checks["mapping_fixture"] = not mapping_failures
        checks["local_claim_verifier"] = not mapping_failures
    elif gate == "rotation":
        rotation_failures = validate_rotation_fixture(Path(spec["fixture"]))  # type: ignore[arg-type]
        rotation_failures.extend(validate_provider_config_marker())
        failures.extend(rotation_failures)
        checks["rotation_fixture"] = not rotation_failures

    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "external_identity_ready": False,
        "boundary": "local external identity prerequisites only; no live OIDC or SAML provider integration claim",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATES))
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    try:
        report = validate(args.gate)
    except RuntimeError as error:
        print(f"external identity gate check failed: {error}", file=sys.stderr)
        return 1
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"external identity {args.gate} check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
