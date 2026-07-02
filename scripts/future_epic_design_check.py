#!/usr/bin/env python3
"""Validate future epic design documents and promotion boundaries."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


EPIC_MARKERS: dict[str, dict[str, object]] = {
    "distributed-consensus": {
        "title": "Production Distributed Consensus",
        "docs": {
            "docs/DISTRIBUTED_CONSENSUS_DESIGN.md": [
                "Failure Model",
                "Consensus State",
                "Replicated Log",
                "Snapshot Install",
                "Membership Changes",
                "Current Evidence Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "Production Distributed Consensus",
                "make distributed-consensus-check",
            ],
            "scripts/consensus_gate_check.py": [
                "production_ready",
                "local consensus evidence only",
            ],
        },
    },
    "managed-cloud": {
        "title": "Managed Cloud",
        "docs": {
            "docs/MANAGED_CLOUD_DESIGN.md": [
                "Control Plane",
                "Data Plane",
                "Tenant Lifecycle",
                "Billing And Quotas",
                "Support And Break-glass",
                "Current Evidence Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "Managed Cloud",
                "make managed-cloud-design-check",
            ],
            "scripts/managed_cloud_gate_check.py": [
                "managed_cloud_ready",
                "local single-node managed-cloud prerequisites only",
            ],
        },
    },
    "enterprise-rbac": {
        "title": "Enterprise RBAC And Compliance",
        "docs": {
            "docs/archive/ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md": [
                "Policy Store",
                "Principal Lifecycle",
                "Quota Model",
                "Tamper-evident Audit",
                "Compliance Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md": [
                "cortexdb.compliance_boundary.v1",
                "Supported certified frameworks today: none.",
            ],
            "docs/archive/RBAC_POLICY_STORE_DESIGN.md": [
                "policy store",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "Enterprise RBAC And Compliance",
                "make rbac-policy-store-check",
            ],
        },
    },
    "hnsw-no-fallback": {
        "title": "Full Production HNSW Without Fallback",
        "docs": {
            "docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md": [
                "Allowed Workloads",
                "Recall SLO",
                "Latency SLO",
                "Graph Freshness",
                "Serving Guardrails",
                "Current Evidence Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "Full Production HNSW Without Fallback",
                "make ann-production-no-fallback-check",
            ],
            "scripts/hnsw_no_fallback_gate_check.py": [
                "fallback_free_general_ready",
                "local no-fallback prerequisite evidence only",
            ],
        },
    },
    "llm-inference": {
        "title": "Built-in LLM Inference",
        "docs": {
            "docs/LLM_INFERENCE_DESIGN.md": [
                "Build-vs-integrate Decision",
                "Provider Interface",
                "ContextPack Boundary",
                "Resource Limits",
                "Safety And Audit",
                "Current Evidence Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "Built-in LLM Inference",
                "make llm-inference-contract-check",
            ],
            "scripts/llm_inference_gate_check.py": [
                "built_in_llm_ready",
                "no production model runtime claim",
            ],
        },
    },
    "external-identity": {
        "title": "External Identity Providers",
        "docs": {
            "docs/EXTERNAL_IDENTITY_DESIGN.md": [
                "Protocol Target",
                "Issuer And Audience",
                "JWKS And Rotation",
                "Role And Scope Mapping",
                "Fail-closed Behavior",
                "Current Evidence Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "External Identity Providers",
                "make oidc-auth-contract-check",
            ],
            "scripts/external_identity_gate_check.py": [
                "external_identity_ready",
                "no live OIDC or SAML provider integration claim",
            ],
        },
    },
    "legal-verification": {
        "title": "Legal-grade Verification",
        "docs": {
            "docs/LEGAL_VERIFICATION_BOUNDARY.md": [
                "Supported Legal Domain",
                "Admissible Sources",
                "Reviewer Workflow",
                "Citation Policy",
                "Output Boundary",
                "Current Evidence Boundary",
                "Required Gates",
                "Non-goals",
            ],
            "docs/FUTURE_NON_GOAL_EPICS.md": [
                "Legal-grade Verification",
                "make legal-verification-dataset-check",
            ],
            "scripts/legal_verification_gate_check.py": [
                "legal_verification_ready",
                "no legal advice or legal-grade certification claim",
            ],
        },
    },
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate_epic(name: str) -> dict[str, object]:
    if name not in EPIC_MARKERS:
        raise RuntimeError(f"unknown epic {name!r}")
    spec = EPIC_MARKERS[name]
    failures: list[str] = []
    docs_checked: list[str] = []
    docs = spec["docs"]
    assert isinstance(docs, dict)
    for file_name, markers in docs.items():
        path = Path(file_name)
        docs_checked.append(file_name)
        if not path.is_file():
            failures.append(f"missing {file_name}")
            continue
        text = read(path)
        for marker in markers:
            if marker not in text:
                failures.append(f"{file_name}: missing marker {marker!r}")
    return {
        "epic": name,
        "title": spec["title"],
        "status": "passed" if not failures else "failed",
        "docs_checked": docs_checked,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--epic",
        action="append",
        choices=sorted(EPIC_MARKERS) + ["all"],
        required=True,
        help="Future epic key to validate. Use --epic all for every epic.",
    )
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def selected_epics(values: list[str]) -> list[str]:
    if "all" in values:
        return sorted(EPIC_MARKERS)
    return values


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        results = [validate_epic(name) for name in selected_epics(args.epic)]
    except RuntimeError as error:
        print(f"future epic design check failed: {error}", file=sys.stderr)
        return 1
    failures = [
        failure
        for result in results
        for failure in result["failures"]
        if isinstance(result["failures"], list)
    ]
    report = {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "epics_checked": [result["epic"] for result in results],
        "results": results,
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"future epic design check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
