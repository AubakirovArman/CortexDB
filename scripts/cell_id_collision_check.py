#!/usr/bin/env python3
"""Validate the agent-scoped cell-id collision gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_CELL_ID_TERMS = [
    "Agent cell-id layout v1",
    "agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence",
    "Top-nibble namespaces reserve bits 63..60",
    "A true 31-bit session/feedback agent slot requires a new persisted schema",
    "AGENT_CELL_ID_SLOT_MASK: u64 = 0x0fff_ffff",
    "CELL_ID_SEQUENCE_MASK: u64 = 0xffff_ffff",
    "pub(crate) fn agent_cell_id_slot(agent_id: AgentId) -> Option<u64>",
    "(agent_id.0 <= AGENT_CELL_ID_SLOT_MASK).then_some(agent_id.0)",
    "pub(crate) fn namespaced_agent_cell_id(",
]

REQUIRED_BOUNDARY_DOC_TERMS = [
    "agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence",
    "Cell ID Layout Boundary",
    "A true 31-bit session/feedback agent slot does not fit this v1 layout",
    "new persisted schema version",
    "migration or refuse-to-read guard",
    "migration compatibility gate",
]

REQUIRED_PRODUCTION_TERMS = {
    "crates/cortex-engine/src/ingestion.rs": [
        "memory_agent_slot(agent_id).ok_or_else(memory_id_overflow)?",
    ],
    "crates/cortex-engine/src/session.rs": [
        "agent_cell_id_slot(agent_id).ok_or_else(session_id_overflow)?",
        "namespaced_agent_cell_id(SESSION_CELL_NAMESPACE, agent_slot, sequence)",
    ],
    "crates/cortex-engine/src/feedback.rs": [
        "agent_cell_id_slot(agent_id).ok_or_else(feedback_id_overflow)?",
        "namespaced_agent_cell_id(FEEDBACK_CELL_NAMESPACE, agent_slot, sequence)",
    ],
}

REQUIRED_TEST_TERMS = {
    "crates/cortex-engine/tests/agent_session_tests.rs": [
        "session_cell_ids_preserve_max_documented_agent_slot",
        "session_cell_ids_reject_agent_slot_overflow",
    ],
    "crates/cortex-engine/tests/feedback_tests.rs": [
        "feedback_cell_ids_preserve_max_documented_agent_slot",
        "feedback_cell_ids_reject_agent_slot_overflow",
    ],
    "crates/cortex-engine/tests/remember_write_contract_tests.rs": [
        "remember_preserves_max_documented_agent_slot",
        "remember_rejects_agent_slot_overflow",
    ],
}

REQUIRED_MAKE_TERMS = [
    "CELL_ID_COLLISION_REPORT ?= target/cell-id-collision/report.json",
    "cell-id-collision-check:",
]

FORBIDDEN_PRODUCTION_TERMS = [
    "agent_id.0 & 0x0fff_ffff",
    "& 0x0fff_ffff",
    "sequence & 0xffff_ffff",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden {term}" for term in terms if term in text]


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    cell_ids = read_text(root / "crates/cortex-engine/src/cell_ids.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures.extend(missing_terms("cell_ids.rs", cell_ids, REQUIRED_CELL_ID_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    gce_contract = read_text(root / "docs/spec/GCE_CONTRACT.md")
    failures.extend(
        missing_terms("docs/spec/GCE_CONTRACT.md", gce_contract, REQUIRED_BOUNDARY_DOC_TERMS)
    )

    for relative, terms in REQUIRED_PRODUCTION_TERMS.items():
        text = read_text(root / relative)
        failures.extend(missing_terms(relative, text, terms))
        failures.extend(forbidden_terms(relative, text, FORBIDDEN_PRODUCTION_TERMS))

    for relative, terms in REQUIRED_TEST_TERMS.items():
        text = read_text(root / relative)
        failures.extend(missing_terms(relative, text, terms))

    return {
        "schema_version": "cortexdb.cell_id_collision.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "layout_version": "agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence",
        "agent_slot_bits": 28,
        "sequence_bits": 32,
        "requires_schema_migration_for_31_bit_slots": True,
        "checked": {
            "cell_id_terms": REQUIRED_CELL_ID_TERMS,
            "boundary_doc_terms": REQUIRED_BOUNDARY_DOC_TERMS,
            "production_terms": REQUIRED_PRODUCTION_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_production_terms": FORBIDDEN_PRODUCTION_TERMS,
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
    print(f"cell-id collision check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
