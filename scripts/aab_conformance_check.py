#!/usr/bin/env python3
"""Validate the CI-safe GCE conformance suite and thin-wrapper reference."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "cortexdb.aab_conformance.report.v1"
FIXTURE_SCHEMA = "cortexdb.gce_conformance.thin_wrapper_reference.v1"

CASES = [
    "scope_widening",
    "fabricated_citation",
    "dropped_conflict",
    "forged_audit_entry",
    "anti_correlation",
    "receipt_verifiability",
    "determinism",
    "plan_binding",
]

REQUIRED_THIN_WRAPPER_FAILED_AXES = [
    "receipt_verifiability",
    "determinism",
    "plan_binding",
]

GATE_MARKERS = [
    "gce-spec-doc-check",
    "receipt-threat-model-check",
    "accountability-receipt-verify-check",
    "accountability-receipt-tamper-check",
    "pack-determinism-hash-check",
    "fail-closed-end-to-end-check",
    "cosine-metric-correctness-check",
    "verification-quality-check",
    "audit-receipt-binding-check",
]

DOC_MARKERS = [
    "# GCE Conformance Suite",
    "## Fast-Lane Cases",
    "## CortexDB Evidence Gates",
    "## Thin-Wrapper Reference",
    "## Pass Criteria",
    "docs/spec/GCE_CONTRACT.md",
    "docs/spec/RECEIPT_VERIFIER.md",
    "thin_wrapper_failed_axis_count >= 3",
]

MAKE_MARKERS = {
    "mk/core.mk": [
        "aab-conformance-check:",
        'python3 scripts/aab_conformance_check.py --root "." --fixture "$(AAB_CONFORMANCE_FIXTURE)" --report "$(AAB_CONFORMANCE_REPORT)"',
    ]
    + [f"$(MAKE) {gate}" for gate in GATE_MARKERS],
    "mk/vars-core.mk": [
        "AAB_CONFORMANCE_FIXTURE ?= fixtures/gce_conformance/thin_wrapper_reference.json",
        "AAB_CONFORMANCE_REPORT ?= target/gce-conformance/report.json",
    ],
    "mk/phony.mk": [
        "aab-conformance-check",
    ],
}


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(read_text(path))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def require_markers(label: str, text: str, markers: list[str]) -> list[str]:
    return [
        f"{label}: missing marker {marker}"
        for marker in markers
        if not contains_marker(text, marker)
    ]


def validate_fixture(path: Path, data: dict[str, Any]) -> tuple[list[str], dict[str, Any]]:
    failures: list[str] = []
    if data.get("schema_version") != FIXTURE_SCHEMA:
        failures.append(f"{path}: schema_version mismatch")

    cortexdb_axes = data.get("cortexdb_axes")
    thin_axes = data.get("thin_wrapper_axes")
    required_failed = data.get("required_failed_axes")
    rationale = data.get("failure_rationale")
    if not isinstance(cortexdb_axes, dict):
        failures.append(f"{path}: cortexdb_axes must be an object")
        cortexdb_axes = {}
    if not isinstance(thin_axes, dict):
        failures.append(f"{path}: thin_wrapper_axes must be an object")
        thin_axes = {}
    if not isinstance(required_failed, list):
        failures.append(f"{path}: required_failed_axes must be an array")
        required_failed = []
    if not isinstance(rationale, dict):
        failures.append(f"{path}: failure_rationale must be an object")
        rationale = {}

    for case in CASES:
        if cortexdb_axes.get(case) != "pass":
            failures.append(f"{path}: CortexDB axis {case} must pass")
        if case not in thin_axes:
            failures.append(f"{path}: thin wrapper axis {case} missing")

    failed_axes = sorted(axis for axis, value in thin_axes.items() if value == "fail")
    for axis in REQUIRED_THIN_WRAPPER_FAILED_AXES:
        if axis not in required_failed:
            failures.append(f"{path}: required_failed_axes missing {axis}")
        if thin_axes.get(axis) != "fail":
            failures.append(f"{path}: thin wrapper axis {axis} must fail")
        if axis not in rationale:
            failures.append(f"{path}: failure_rationale missing {axis}")

    if len(failed_axes) < 3:
        failures.append(f"{path}: thin wrapper must fail at least 3 axes")

    summary = {
        "cortexdb_passed_all": all(cortexdb_axes.get(case) == "pass" for case in CASES),
        "thin_wrapper_failed_axes": failed_axes,
        "thin_wrapper_failed_axis_count": len(failed_axes),
        "thin_wrapper_failed_required_axes": [
            axis for axis in REQUIRED_THIN_WRAPPER_FAILED_AXES if thin_axes.get(axis) == "fail"
        ],
    }
    return failures, summary


def validate(root: Path, fixture_path: Path) -> dict[str, Any]:
    failures: list[str] = []

    try:
        doc = read_text(root / "docs/spec/GCE_CONFORMANCE.md")
    except FileNotFoundError:
        doc = ""
        failures.append("docs/spec/GCE_CONFORMANCE.md: missing file")
    failures.extend(require_markers("GCE_CONFORMANCE.md", doc, DOC_MARKERS + CASES + GATE_MARKERS))

    for relative, markers in MAKE_MARKERS.items():
        try:
            text = read_text(root / relative)
        except FileNotFoundError:
            failures.append(f"{relative}: missing file")
            continue
        failures.extend(require_markers(relative, text, markers))

    try:
        fixture = load_json(fixture_path)
        fixture_failures, summary = validate_fixture(fixture_path, fixture)
        failures.extend(fixture_failures)
    except (OSError, ValueError) as error:
        fixture = {}
        summary = {
            "cortexdb_passed_all": False,
            "thin_wrapper_failed_axes": [],
            "thin_wrapper_failed_axis_count": 0,
            "thin_wrapper_failed_required_axes": [],
        }
        failures.append(str(error))

    return {
        "schema_version": SCHEMA_VERSION,
        "status": "failed" if failures else "passed",
        "fixture": str(fixture_path),
        "cases": CASES,
        "evidence_gates": GATE_MARKERS,
        "thin_wrapper_reference": fixture.get("name"),
        "cortexdb_passed_all": summary["cortexdb_passed_all"],
        "thin_wrapper_failed_axes": summary["thin_wrapper_failed_axes"],
        "thin_wrapper_failed_axis_count": summary["thin_wrapper_failed_axis_count"],
        "thin_wrapper_failed_required_axes": summary["thin_wrapper_failed_required_axes"],
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--fixture", required=True, help="thin-wrapper reference fixture")
    parser.add_argument("--report", required=True, help="output JSON report")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    report = validate(root, root / args.fixture)
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
    print(f"AAB conformance check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
