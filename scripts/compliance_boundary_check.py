#!/usr/bin/env python3
"""Validate CortexDB's local compliance boundary documentation."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from compliance_certification_evidence import validate_compliance_certification_evidence
from evidence_origin import is_operator_origin_validation
from receipt_production_component_origin import (
    add_production_origin_args,
    missing_production_origin_inputs,
    trust_anchor_ready,
    validate_component_trust_anchor,
)


REQUIRED_MARKERS = {
    "boundary_schema": [
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb.compliance_boundary.v1"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "Supported certified frameworks today: none."),
        (
            "docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md",
            "cortexdb.compliance_certification_evidence.v1",
        ),
    ],
    "non_claims": [
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "SOC 2 compliance"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "ISO 27001 certification"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "HIPAA compliance"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "GDPR processor or controller compliance"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "legal-grade fact verification"),
    ],
    "local_evidence_controls": [
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "CORTEXDB_AUTH_POLICY_STORE_FILE"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb auth-review"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb audit --verify-chain"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb audit-export-siem"),
        ("docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md", "make public-claims-check"),
    ],
    "security_docs_boundary": [
        ("docs/archive/SECURITY_HARDENING_EVIDENCE.md", "It does not claim full RBAC"),
        ("docs/archive/SECURITY_BETA_BASELINE.md", "It does not mean CortexDB has external security certification"),
        ("docs/archive/ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md", "must not imply certification"),
        ("docs/FUTURE_NON_GOAL_EPICS.md", "Public docs state exactly which compliance frameworks are supported"),
        ("docs/SECURITY_MODEL.md", "COMPLIANCE_CERTIFICATION_EVIDENCE"),
    ],
}

def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate(args: argparse.Namespace) -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok

    certification = {
        "provided": False,
        "valid": False,
        "summary": {},
        "failures": [],
    }
    trust_anchor = {"provided": False, "valid": False, "summary": {}, "failures": []}
    missing_origin_inputs: list[str] = []
    if args.certification_evidence:
        missing_origin_inputs = missing_production_origin_inputs(args)
        for env_name in missing_origin_inputs:
            failures.append(f"missing required input for compliance certification claim: {env_name}")
        trust_anchor = validate_component_trust_anchor(args)
        failures.extend(
            f"production origin trust anchor evidence: {failure}"
            for failure in trust_anchor["failures"]
        )
        certification = validate_compliance_certification_evidence(
            Path(args.certification_evidence),
            expected_framework=args.expected_framework,
            expected_key_attestor_key_id=args.expected_key_attestor_key_id,
            expected_key_attestor_public_key_hex=args.expected_key_attestor_public_key_hex,
            expected_key_attestor_ref=args.expected_key_attestor_ref,
            expected_key_attestor_public_key_ref=args.expected_key_attestor_public_key_ref,
            require_production_origin_proof=True,
        )
        if not certification["valid"]:
            for failure in certification["failures"]:
                failures.append(f"certification evidence: {failure}")

    has_operator_certification = (
        not missing_origin_inputs
        and trust_anchor_ready(trust_anchor)
        and is_operator_origin_validation(certification)
        and certification.get("production_origin_proof_required") is True
        and certification.get("production_origin_proof_valid") is True
    )

    supported_frameworks = []
    if has_operator_certification:
        supported_frameworks.append(certification["summary"]["framework"])

    blockers = []
    if not has_operator_certification:
        blocker_id = "external_compliance_certification_not_implemented"
        required_gate = "operator supplies external compliance certification evidence"
        if missing_origin_inputs:
            blocker_id = "external_compliance_certification_missing_origin_inputs"
            required_gate = (
                "compliance certification claim requires expected "
                "key-attestor trust anchor and publisher inputs"
            )
        elif trust_anchor.get("valid") is not True and trust_anchor.get("provided") is True:
            blocker_id = "external_compliance_certification_trust_anchor_invalid"
            required_gate = (
                "compliance certification claim requires valid production-origin "
                "trust-anchor publication evidence"
            )
        elif trust_anchor.get("provided") is True and not trust_anchor_ready(trust_anchor):
            blocker_id = "external_compliance_certification_trust_anchor_not_operator_origin"
            required_gate = (
                "compliance certification claim requires operator-origin "
                "trust-anchor publication evidence"
            )
        elif certification.get("valid") is True:
            blocker_id = "external_compliance_certification_not_operator_origin"
            required_gate = "valid compliance evidence must be operator-origin, not fixture/generated/local evidence"
        elif certification.get("provided") is True:
            blocker_id = "external_compliance_certification_invalid"
            required_gate = (
                "valid compliance evidence must include production_origin_proof "
                "bound to expected key-attestor inputs and valid trust-anchor "
                "publication evidence"
            )
        blockers.append(
            {
                "id": blocker_id,
                "status": "blocked",
                "required_gate": required_gate,
            }
        )

    claim_boundary = (
        "operator-supplied external compliance certification evidence accepted; "
        "public docs still do not claim certification without operator evidence"
        if has_operator_certification
        else "local evidence only; no external compliance certification"
    )
    return {
        "schema_version": "cortexdb.compliance_boundary_check.v1",
        "status": "passed" if not failures else "failed",
        "supported_certified_frameworks": supported_frameworks,
        "production_origin_trust_anchor": trust_anchor,
        "external_certification": certification,
        "compliance_immutability": has_operator_certification,
        "claim_boundary": claim_boundary,
        "blockers": blockers,
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    parser.add_argument("--certification-evidence")
    parser.add_argument("--expected-framework")
    add_production_origin_args(parser)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(args)
    except RuntimeError as error:
        print(f"compliance boundary check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"compliance boundary check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
