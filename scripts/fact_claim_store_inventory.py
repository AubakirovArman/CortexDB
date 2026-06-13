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
    path = "crates/cortex-engine/src/typed_body.rs"
    needles = ["pub struct FactBody", "metric", "value", "currency", "project"]
    text = read(path)
    status = "partial" if has_all(text, needles) else "fail"
    return Check(
        "typed_fact_body_shape",
        status,
        line_refs(path, needles),
        "FactBody stores numeric value as text; B07 needs a typed NumericValue claim record.",
    )


def check_verify_numeric_scan_path() -> Check:
    path = "crates/cortex-engine/src/verification/guards.rs"
    needles = [
        "numeric_mismatch_details",
        "CellMetadata::from_payload(payload)",
        "extract_numeric_values(&metadata.body_text)",
    ]
    text = read(path)
    status = "partial" if has_all(text, needles) else "pass"
    gap = (
        "VERIFY numeric conflict detection still reparses payload body per evidence item."
        if status == "partial"
        else None
    )
    return Check("verify_numeric_scan_path", status, line_refs(path, needles), gap)


def check_support_numeric_scan_path() -> Check:
    path = "crates/cortex-engine/src/verification/support.rs"
    needles = [
        "numeric_entailment",
        "CellMetadata::from_payload(payload).body_text",
        "extract_numeric_values(&payload_text)",
    ]
    text = read(path)
    status = "partial" if has_all(text, needles) else "pass"
    gap = (
        "Numeric support entailment still reparses payload body instead of consulting typed claims."
        if status == "partial"
        else None
    )
    return Check("support_numeric_scan_path", status, line_refs(path, needles), gap)


def check_typed_fact_store() -> Check:
    paths = [
        "crates/cortex-engine/src/verification.rs",
        "crates/cortex-engine/src/verification/numeric/mod.rs",
    ]
    evidence: list[str] = []
    found = False
    for path in paths:
        text = read(path)
        needles = ["FactClaimStore", "NumericFactStore", "TypedFactStore"]
        found = found or any(needle in text for needle in needles)
        evidence.extend(line_refs(path, needles))
    status = "pass" if found else "fail"
    return Check(
        "typed_fact_store",
        status,
        evidence,
        None if found else "No maintained typed fact/claim store is wired into verification yet.",
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
            "Add a conservative typed fact/claim record backed by NumericValue.",
            "Populate a maintained in-memory fact store on open/put/patch/tombstone.",
            "Route VERIFY numeric conflict and support checks through typed claims where possible.",
            "Keep parser-path fallback until parity tests prove safe replacement.",
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
