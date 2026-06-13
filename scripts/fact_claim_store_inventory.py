#!/usr/bin/env python3
"""Inventory typed numeric fact/claim store coverage for EPIC-B07."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Check:
    name: str
    status: str
    evidence: list[str]
    gap: str | None = None

    def to_json(self) -> dict[str, object]:
        value: dict[str, object] = {
            "name": self.name,
            "status": self.status,
            "evidence": self.evidence,
        }
        if self.gap:
            value["gap"] = self.gap
        return value


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def line_refs(path: str, needles: list[str]) -> list[str]:
    refs: list[str] = []
    for index, line in enumerate(read(path).splitlines(), start=1):
        if any(needle in line for needle in needles):
            refs.append(f"{path}:{index}")
    return refs


def has_all(text: str, needles: list[str]) -> bool:
    return all(needle in text for needle in needles)


def check_numeric_value_model() -> Check:
    path = "crates/cortex-engine/src/verification/numeric/value.rs"
    needles = [
        "pub struct NumericValue",
        "scaled_value",
        "currency",
        "unit",
        "Magnitude",
        "NumericComparison",
        "compare_numeric_values",
    ]
    text = read(path)
    status = "pass" if has_all(text, needles) else "fail"
    return Check(
        "numeric_value_model",
        status,
        line_refs(path, needles),
        None if status == "pass" else "Numeric values are not typed enough for fact claims.",
    )


def check_numeric_parser() -> Check:
    path = "crates/cortex-engine/src/verification/numeric/parse.rs"
    needles = [
        "extract_numeric_values",
        "parse_currency_code",
        "parse_magnitude_suffix",
        "parse_unit_code",
    ]
    text = read(path)
    status = "pass" if has_all(text, needles) else "fail"
    return Check(
        "numeric_parser",
        status,
        line_refs(path, needles),
        None if status == "pass" else "No deterministic numeric parser exists.",
    )


def check_typed_fact_body_shape() -> Check:
    paths = [
        "crates/cortex-engine/src/typed_body.rs",
        "crates/cortex-engine/src/verification/numeric/fact_claim.rs",
    ]
    needles = {
        "crates/cortex-engine/src/typed_body.rs": [
            "pub struct FactBody",
            "metric",
            "value",
            "currency",
            "project",
        ],
        "crates/cortex-engine/src/verification/numeric/fact_claim.rs": [
            "NumericFactRecord",
            "FactBody::parse",
            "single_numeric_value",
            "NumericValue",
        ],
    }
    evidence: list[str] = []
    ok = True
    for path in paths:
        text = read(path)
        ok = ok and has_all(text, needles[path])
        evidence.extend(line_refs(path, needles[path]))
    status = "pass" if ok else "fail"
    return Check(
        "typed_fact_body_shape",
        status,
        evidence,
        None if ok else "FactBody numeric values are not materialized into typed claim records.",
    )


def check_verify_numeric_scan_path() -> Check:
    guards_path = "crates/cortex-engine/src/verification/guards.rs"
    execution_path = "crates/cortex-engine/src/verification/execution.rs"
    store_path = "crates/cortex-engine/src/verification/numeric/fact_claim.rs"
    fallback_needles = [
        "numeric_mismatch_details",
        "CellMetadata::from_payload(payload)",
        "extract_numeric_values(&metadata.body_text)",
    ]
    typed_needles = ["add_verify_matches", "VerificationNumericConflict", "numeric_conflict"]
    execution_needles = ["fact_claim_store.add_verify_matches"]
    fallback_text = read(guards_path)
    typed_text = read(store_path)
    execution_text = read(execution_path)
    typed_ok = has_all(typed_text, typed_needles) and has_all(execution_text, execution_needles)
    fallback_exists = has_all(fallback_text, fallback_needles)
    evidence = (
        line_refs(store_path, typed_needles)
        + line_refs(execution_path, execution_needles)
        + line_refs(guards_path, fallback_needles)
    )
    status = "pass" if typed_ok else ("partial" if fallback_exists else "fail")
    gap = None if typed_ok else "VERIFY numeric conflict checks do not consult typed claims yet."
    return Check("verify_numeric_scan_path", status, evidence, gap)


def check_support_numeric_scan_path() -> Check:
    support_path = "crates/cortex-engine/src/verification/support.rs"
    execution_path = "crates/cortex-engine/src/verification/execution.rs"
    store_path = "crates/cortex-engine/src/verification/numeric/fact_claim.rs"
    fallback_needles = [
        "numeric_entailment",
        "CellMetadata::from_payload(payload).body_text",
        "extract_numeric_values(&payload_text)",
    ]
    typed_needles = ["add_verify_matches", "NumericEntailment", "normalized_numeric_equal"]
    execution_needles = ["fact_claim_store.add_verify_matches"]
    fallback_text = read(support_path)
    typed_text = read(store_path)
    execution_text = read(execution_path)
    typed_ok = has_all(typed_text, typed_needles) and has_all(execution_text, execution_needles)
    fallback_exists = has_all(fallback_text, fallback_needles)
    evidence = (
        line_refs(store_path, typed_needles)
        + line_refs(execution_path, execution_needles)
        + line_refs(support_path, fallback_needles)
    )
    status = "pass" if typed_ok else ("partial" if fallback_exists else "fail")
    gap = None if typed_ok else "VERIFY numeric support checks do not consult typed claims yet."
    return Check("support_numeric_scan_path", status, evidence, gap)


def check_typed_fact_store() -> Check:
    paths = [
        "crates/cortex-engine/src/verification/numeric/mod.rs",
        "crates/cortex-engine/src/verification/numeric/fact_claim.rs",
        "crates/cortex-engine/src/database/stores.rs",
        "crates/cortex-engine/src/database/open.rs",
        "crates/cortex-engine/src/database/write.rs",
        "crates/cortex-engine/src/replication/install.rs",
    ]
    evidence: list[str] = []
    ok = True
    for path in paths:
        text = read(path)
        if path.endswith("numeric/mod.rs"):
            needles = ["fact_claim"]
        elif path.endswith("fact_claim.rs"):
            needles = ["FactClaimStore", "record_from_payload", "apply_record", "apply_tombstone"]
        elif path.endswith("database/stores.rs"):
            needles = [
                "fact_claim_store",
                "FactClaimStore::from_memtable",
                "FactClaimStore::record_from_payload",
                "apply_derived_cell_record",
                "apply_derived_tombstone",
            ]
        elif path.endswith("database/open.rs"):
            needles = ["DerivedStores::from_memtable_for_residency", "fact_claim_store"]
        elif path.endswith("database/write.rs"):
            needles = ["apply_derived_cell_record", "apply_derived_tombstone"]
        else:
            needles = ["rebuild_derived_stores_from_memtable"]
        ok = ok and has_all(text, needles)
        evidence.extend(line_refs(path, needles))
    status = "pass" if ok else "fail"
    return Check(
        "typed_fact_store",
        status,
        evidence,
        None if ok else "FactClaimStore is not fully wired into Database open/write lifecycle.",
    )


def inventory() -> dict[str, object]:
    checks = [
        check_numeric_value_model(),
        check_numeric_parser(),
        check_typed_fact_body_shape(),
        check_verify_numeric_scan_path(),
        check_support_numeric_scan_path(),
        check_typed_fact_store(),
    ]
    gaps = [check.gap for check in checks if check.gap]
    failing = [check.name for check in checks if check.status == "fail"]
    partial = [check.name for check in checks if check.status == "partial"]
    status = "complete" if not failing and not partial else "partial"
    return {
        "schema_version": "cortexdb.fact_claim_store_inventory.v1",
        "epic": "EPIC-B07",
        "status": status,
        "summary": {
            "checks": len(checks),
            "pass": sum(1 for check in checks if check.status == "pass"),
            "partial": len(partial),
            "fail": len(failing),
        },
        "checks": [check.to_json() for check in checks],
        "remaining_gaps": gaps,
        "next_patch_order": [
            "Keep parser-path fallback until broader parity coverage allows removal.",
            "Move to EPIC-B08 after B07 tests and docs are green.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/fact-claim-store/inventory.json")
    args = parser.parse_args()
    output = inventory()
    report = Path(args.report)
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = output["summary"]
    print(
        "fact claim store inventory: "
        f"status={output['status']} "
        f"pass={summary['pass']} partial={summary['partial']} fail={summary['fail']}"
    )
    print(f"report: {report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
