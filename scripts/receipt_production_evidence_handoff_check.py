#!/usr/bin/env python3
"""Validate production evidence handoff consistency with validator constants."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from compliance_certification_evidence import (
    EVIDENCE_SCHEMA as COMPLIANCE_EVIDENCE_SCHEMA,
)
from evidence_origin import (
    PRODUCTION_ORIGIN_KEY_ATTESTATION_SCHEMA,
    PRODUCTION_ORIGIN_KEY_ATTESTATION_SIGNING_DOMAIN,
    PRODUCTION_ORIGIN_PROOF_SCHEMA,
    PRODUCTION_ORIGIN_STATEMENT_SIGNING_DOMAIN,
    PRODUCTION_ORIGIN_STATEMENT_SCHEMA,
)
from receipt_kms_hsm_evidence import (
    EVIDENCE_SCHEMA as KMS_HSM_EVIDENCE_SCHEMA,
    REQUEST_SCHEMA,
    RESPONSE_SCHEMA,
    SIGNING_DOMAIN,
)
from receipt_production_evidence_handoff_payload import operator_handoff
from receipt_production_evidence_preflight import REQUIRED_INPUTS
from receipt_production_origin_trust_anchor_evidence import (
    EVIDENCE_SCHEMA as TRUST_ANCHOR_EVIDENCE_SCHEMA,
)

try:
    from receipt_production_evidence_handoff_checklists import check_checklist, require
except ModuleNotFoundError:
    from scripts.receipt_production_evidence_handoff_checklists import check_checklist, require


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
