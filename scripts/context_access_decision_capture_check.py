#!/usr/bin/env python3
"""Validate captured ContextPack access-decision wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_SOURCE_TERMS = [
    "pub struct CapturedAccessDecision",
    "captured_access_decision: Option<CapturedAccessDecision>",
    "captured_access_decision_for_candidate",
    "captured_allowed_access_decision",
    "CAPTURED_ACCESS_POLICY_VERSION",
    "agent_view_digest",
    "cortexdb.agent_view.digest.v1",
    "blake3_256_domain",
    "survived AQL permission filtering",
    "pub struct CapturedAccessDenial",
    "pub struct CapturedAccessDenialSet",
    "captured_access_denial_for_candidate",
    "captured_denied_access_decision",
    "MAX_CAPTURED_ACCESS_DENIALS",
    "cortexdb.accountability.access_denial.v1",
    "cortexdb.accountability.access_denial.cell_id_hash.v1",
    "cortexdb.accountability.access_denial.evidence_digest.v1",
    "captured_access_denials: CapturedAccessDenialSet",
    "AgentAllowedBypassProvider",
    "BitmapOp::PushAgentAllowed",
    "capture_agent_allowed_bitmap_denials(plan, provider, &candidates)",
    "capture_agent_allowed_bitmap_denials(plan, provider, &bitmap_candidates)",
    "capture_access_denials(provider, &permission_input, &candidates)",
    "capture_access_denials(provider, &bitmap_candidates, &permission_candidates)",
    "rejected by AQL agent access filtering",
]

REQUIRED_CONTEXT_TERMS = [
    "policy_version: Option<String>",
    "agent_view_digest: Option<String>",
    "cell.captured_access_decision.as_ref()",
    '"policy_version"',
    '"agent_view_digest"',
]

REQUIRED_TEST_TERMS = [
    "aql_context_pack_uses_captured_access_decision_from_retrieval_path",
    "survived AQL permission filtering",
    "agent_view_readable_scope.v1",
    "assert!(!decision.reason.contains(\"re-derived\"))",
    "retrieve_execution_report_captures_permission_denials_without_forbidden_payload",
    "secret-denied-payload-marker",
    "assert!(!denial_debug.contains(\"secret-denied-payload-marker\"))",
    "assert!(!denial_debug.contains(\"scope=default\"))",
    "assert!(!denial_debug.contains(\"CellId(2)\"))",
]

REQUIRED_CONTRACT_TERMS = [
    '"policy_version"',
    '"agent_view_digest"',
    "policy_version:",
    "agent_view_digest:",
]

REQUIRED_MAKE_TERMS = [
    "CONTEXT_ACCESS_DECISION_CAPTURE_REPORT ?= target/context-access-decision-capture/report.json",
    "context-access-decision-capture-check:",
    "cargo test -p cortex-engine --lib retrieve_execution_report_captures_permission_denials_without_forbidden_payload",
    "cargo test -p cortex-engine --test context_access_decision_capture --all-features",
    'python3 scripts/context_access_decision_capture_check.py --root "." --report "$(CONTEXT_ACCESS_DECISION_CAPTURE_REPORT)"',
]

REQUIRED_PHONY_TERMS = [
    "context-access-decision-capture-check",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    source = "\n".join(
        read_text(root / path)
        for path in [
            "crates/cortex-engine/src/access_capture.rs",
            "crates/cortex-engine/src/database/types.rs",
            "crates/cortex-engine/src/database/candidates.rs",
            "crates/cortex-engine/src/query/provider.rs",
            "crates/cortex-engine/src/exec/scans.rs",
            "crates/cortex-engine/src/exec/retrieve.rs",
            "crates/cortex-engine/src/exec/retrieve/hybrid.rs",
        ]
    )
    context = "\n".join(
        read_text(root / path)
        for path in [
            "crates/cortex-engine/src/context/mod.rs",
            "crates/cortex-engine/src/context/pack/access.rs",
            "crates/cortex-engine/src/context/pack/builder.rs",
            "crates/cortex-engine/src/context/export/json_export.rs",
            "crates/cortex-engine/src/canonical/mod.rs",
        ]
    )
    tests = "\n".join(
        read_text(root / path)
        for path in [
            "crates/cortex-engine/tests/context_access_decision_capture.rs",
            "crates/cortex-engine/src/database/tests.rs",
        ]
    )
    contracts = "\n".join(
        read_text(root / path)
        for path in [
            "docs/schemas/context_pack.v1.json",
            "docs/openapi.yaml",
            "crates/cortex-server/src/responses/context.rs",
            "crates/cortex-sdk/src/types/context.rs",
        ]
    )
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")

    failures: list[str] = []
    failures.extend(missing_terms("captured access source", source, REQUIRED_SOURCE_TERMS))
    failures.extend(missing_terms("context access surface", context, REQUIRED_CONTEXT_TERMS))
    failures.extend(missing_terms("context_access_decision_capture.rs", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("contracts", contracts, REQUIRED_CONTRACT_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", phony, REQUIRED_PHONY_TERMS))

    return {
        "schema_version": "cortexdb.context_access_decision_capture.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "source_terms": REQUIRED_SOURCE_TERMS,
            "context_terms": REQUIRED_CONTEXT_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "contract_terms": REQUIRED_CONTRACT_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "phony_terms": REQUIRED_PHONY_TERMS,
        },
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"context access decision capture check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
