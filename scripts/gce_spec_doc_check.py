#!/usr/bin/env python3
"""Validate the normative GCE contract document and gate wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "cortexdb.gce_spec_doc.report.v1"

DOC_MARKERS = [
    "# Governed Context Engine Contract",
    "Status: normative category contract",
    "## Scope And Compatibility",
    "## ContextPack Result Type",
    "## Verification Output",
    "## Six GCE Invariants",
    "## Conformance Obligations",
    "## Current CortexDB Gate Evidence",
    "## Non-Goals And Claim Boundaries",
    "docs/schemas/context_pack.v1.json",
    "docs/spec/ACCOUNTABILITY_RECEIPT_V1.md",
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md",
]

INVARIANT_MARKERS = [
    "Invariant 1 - Compiled governed context",
    "Invariant 2 - Deterministic LLM-free Q16 governance",
    "Invariant 3 - Fail-closed plan algebra",
    "Invariant 4 - Provenance and verification are first-class outputs",
    "Invariant 5 - Conflict preservation, not LWW",
    "Invariant 6 - TTL/decay participates in ranking",
]

CONTEXT_FIELD_MARKERS = [
    "ContextPack",
    "ContextPackCell",
    "ContextAccessDecision",
    "ContextSpanProvenance",
    "ContextPackAnomaly",
    "token_budget_tokens",
    "estimated_tokens",
    "truncated",
    "citations_required",
    "answerability_q16",
    "conflict_visibility_q16",
    "visible_conflict_count",
    "cells",
    "anomalies",
    "grounding_report",
    "cell_id",
    "payload",
    "metadata",
    "citation",
    "provenance",
    "explain",
    "access_decision",
    "policy_version",
    "agent_view_digest",
    "source_byte_start",
    "source_byte_end",
    "source_ref",
    "retrieval_incomplete",
]

VERIFICATION_MARKERS = [
    "VerificationReport",
    "VerificationEvidence",
    "VerificationGuard",
    "VerificationNumericConflict",
    "status",
    "confidence_q16",
    "evidence",
    "contradicting_evidence",
    "guards",
    "numeric_conflicts",
    "numeric",
    "temporal",
    "citation",
]

CONFORMANCE_MARKERS = [
    "MUST emit `context_pack.v1`",
    "MUST preserve additive compatibility",
    "MUST enforce fail-closed access",
    "MUST use deterministic integer or Q16 governance",
    "MUST expose `ContextPack` cells",
    "MUST expose `VERIFY FACT` status",
    "MUST preserve conflict signals",
    "MUST surface retrieval or grounding incompleteness",
    "MUST keep optional receipt material additive",
    "MUST NOT require an LLM",
    "MUST NOT claim production-grade external transparency",
    "MUST NOT claim KMS/HSM custody",
    "MUST NOT treat application-side post-filtering",
]

SOURCE_MARKERS = {
    "crates/cortex-engine/src/context/mod.rs": CONTEXT_FIELD_MARKERS,
    "crates/cortex-engine/src/verification/types.rs": VERIFICATION_MARKERS,
    "crates/cortex-aql/src/binder.rs": [
        "PushAgentAllowed",
        "PushLive",
        "BitmapOp::And",
        "where_clause",
    ],
    "docs/schemas/context_pack.v1.json": [
        "context_pack.v1",
        "grounding_report",
        "accountability_receipt",
        "retrieval_incomplete",
    ],
    "docs/spec/ACCOUNTABILITY_RECEIPT_V1.md": [
        "accountability_receipt.v1",
        "cortex-receipt-verify",
        "blake3-256",
        "ed25519",
    ],
}

MAKE_MARKERS = {
    "mk/core.mk": [
        "gce-spec-doc-check:",
        'python3 scripts/gce_spec_doc_check.py --root "." --report "$(GCE_SPEC_DOC_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "GCE_SPEC_DOC_REPORT ?= target/gce-spec/doc-report.json",
    ],
    "mk/phony.mk": [
        "gce-spec-doc-check",
    ],
}


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def require_markers(label: str, text: str, markers: list[str]) -> list[str]:
    return [
        f"{label}: missing marker {marker}"
        for marker in markers
        if not contains_marker(text, marker)
    ]


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    doc_path = root / "docs/spec/GCE_CONTRACT.md"

    try:
        doc = read_text(doc_path)
    except FileNotFoundError:
        doc = ""
        failures.append("docs/spec/GCE_CONTRACT.md: missing file")

    failures.extend(require_markers("GCE_CONTRACT.md", doc, DOC_MARKERS))
    failures.extend(require_markers("GCE_CONTRACT.md invariants", doc, INVARIANT_MARKERS))
    failures.extend(require_markers("GCE_CONTRACT.md context fields", doc, CONTEXT_FIELD_MARKERS))
    failures.extend(require_markers("GCE_CONTRACT.md verification", doc, VERIFICATION_MARKERS))
    failures.extend(require_markers("GCE_CONTRACT.md conformance", doc, CONFORMANCE_MARKERS))

    for relative, markers in SOURCE_MARKERS.items():
        try:
            source = read_text(root / relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        failures.extend(require_markers(relative, source, markers))
        for marker in markers:
            if not contains_marker(doc, marker):
                failures.append(f"GCE_CONTRACT.md: missing source term {marker}")

    for relative, markers in MAKE_MARKERS.items():
        try:
            text = read_text(root / relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        failures.extend(require_markers(relative, text, markers))

    return {
        "schema_version": SCHEMA_VERSION,
        "status": "failed" if failures else "passed",
        "spec": "docs/spec/GCE_CONTRACT.md",
        "checked_sections": DOC_MARKERS,
        "checked_invariants": INVARIANT_MARKERS,
        "checked_context_fields": CONTEXT_FIELD_MARKERS,
        "checked_verification_fields": VERIFICATION_MARKERS,
        "checked_conformance_terms": CONFORMANCE_MARKERS,
        "checked_source_files": sorted(SOURCE_MARKERS),
        "checked_make_files": sorted(MAKE_MARKERS),
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"GCE spec doc check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
