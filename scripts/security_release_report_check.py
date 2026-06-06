#!/usr/bin/env python3
"""Validate the per-release CortexDB security hardening report."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REPORT_MARKERS = {
    "release_report": [
        "Release Security Hardening Report",
        "make security-release-report-check",
        "target/security-release/report.json",
    ],
    "release_gates": [
        "make security-gate-v2-check",
        "target/security-gate-v2/report.json",
        "make compliance-boundary-check",
        "target/compliance-boundary/report.json",
    ],
    "remaining_risks": [
        "Remaining Risks",
        "external identity",
        "enterprise compliance certification",
        "managed-cloud security",
        "distributed authorization correctness",
        "KMS-backed encrypted backup custody",
        "provider-backed object-store backup",
    ],
}
CHECKLIST_MARKERS = [
    "make security-gate-v2-check",
    "make security-release-report-check",
]


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def read_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"failed to parse {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def validate_markers(
    label: str, text: str, markers: list[str], failures: list[str]
) -> bool:
    ok = True
    for marker in markers:
        if marker not in text:
            failures.append(f"{label}: missing marker {marker!r}")
            ok = False
    return ok


def validate(args: argparse.Namespace) -> dict[str, Any]:
    security_gate = read_report(Path(args.security_gate_v2_report))
    compliance = read_report(Path(args.compliance_boundary_report))
    evidence = read_text(Path("docs/SECURITY_HARDENING_EVIDENCE.md"))
    checklist = read_text(Path("docs/SECURITY_RELEASE_CHECKLIST.md"))

    failures: list[str] = []
    if security_gate.get("status") != "passed":
        failures.append("security-gate-v2 report status is not passed")
    if compliance.get("status") != "passed":
        failures.append("compliance-boundary report status is not passed")

    marker_results: dict[str, bool] = {}
    for group, markers in REPORT_MARKERS.items():
        marker_results[group] = validate_markers(
            f"SECURITY_HARDENING_EVIDENCE.md/{group}", evidence, markers, failures
        )
    marker_results["release_checklist"] = validate_markers(
        "SECURITY_RELEASE_CHECKLIST.md", checklist, CHECKLIST_MARKERS, failures
    )

    component_status = {
        "security_gate_v2": security_gate.get("status") == "passed",
        "compliance_boundary": compliance.get("status") == "passed",
    }
    return {
        "schema_version": "cortexdb.security_release_report.v1",
        "status": "passed" if not failures else "failed",
        "component_status": component_status,
        "marker_results": marker_results,
        "reports": {
            "security_gate_v2": args.security_gate_v2_report,
            "compliance_boundary": args.compliance_boundary_report,
            "security_release": args.report,
        },
        "remaining_risk_boundary": {
            "external_identity": "not claimed",
            "enterprise_compliance_certification": "not claimed",
            "managed_cloud_security": "not claimed",
            "distributed_authorization_correctness": "not claimed",
            "kms_backed_backup_custody": "not claimed",
            "provider_object_store_backup": "not claimed",
        },
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--security-gate-v2-report", required=True)
    parser.add_argument("--compliance-boundary-report", required=True)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(args)
    except RuntimeError as error:
        print(f"security release report check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"security release report check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
