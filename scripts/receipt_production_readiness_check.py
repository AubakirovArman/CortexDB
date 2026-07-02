#!/usr/bin/env python3
"""Aggregate public receipt production-readiness evidence and blockers."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


SUCCESS_STATUSES = {"passed", "ok"}

REQUIRED_DOC_MARKERS = {
    "docs/SECURITY_MODEL.md": [
        "receipt-production-readiness-check",
        "receipt-production-ready-check",
        "receipt-kms-hsm-custody-check",
        "production_ready=false",
        "KMS/HSM-backed receipt key custody",
        "COMPLIANCE_CERTIFICATION_EVIDENCE",
        "operator-origin evidence",
        "synthetic validator coverage",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "receipt-production-readiness-check",
        "receipt-production-ready-check",
        "receipt-kms-hsm-custody-check",
        "production-grade public receipt",
        "operator-origin evidence",
    ],
}


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


def report_passed(report: dict[str, Any]) -> bool:
    return report.get("status") in SUCCESS_STATUSES


def preflight_ready(report: dict[str, Any] | None) -> bool:
    if not isinstance(report, dict):
        return False
    basic_ready = (
        report_passed(report)
        and report.get("production_evidence_ready") is True
        and report.get("production_origin_proof_required") is True
    )
    if not basic_ready:
        return False
    readiness = report.get("readiness")
    if not isinstance(readiness, dict):
        return False
    if (
        readiness.get("production_origin_trust_anchor") is not True
        or readiness.get("kms_hsm_operator_evidence") is not True
        or readiness.get("compliance_operator_evidence") is not True
    ):
        return False
    evidence = report.get("operator_evidence")
    if not isinstance(evidence, dict):
        return False
    return (
        evidence_is_operator(evidence.get("production_origin_trust_anchor"))
        and evidence_has_required_origin_proof(evidence.get("receipt_kms_hsm_custody"))
        and evidence_has_required_origin_proof(evidence.get("compliance_certification"))
    )


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def doc_failures(root: Path) -> list[str]:
    failures: list[str] = []
    for relative, markers in REQUIRED_DOC_MARKERS.items():
        text = read_text(root / relative)
        for marker in markers:
            if marker not in text:
                failures.append(f"{relative}: missing marker {marker!r}")
    return failures


def compliance_ready(report: dict[str, Any]) -> bool:
    frameworks = report.get("supported_certified_frameworks")
    certification = report.get("external_certification")
    if isinstance(certification, dict):
        return (
            certification.get("valid") is True
            and evidence_is_operator(certification)
            and evidence_has_required_origin_proof(certification)
            and component_trust_anchor_ready(report)
            and isinstance(frameworks, list)
            and bool(frameworks)
            and report.get("compliance_immutability") is True
        )
    return isinstance(frameworks, list) and bool(frameworks)


def kms_hsm_ready(report: dict[str, Any]) -> bool:
    evidence = report.get("operator_evidence")
    return (
        report.get("kms_hsm_custody") is True
        and evidence_is_operator(evidence)
        and evidence_has_required_origin_proof(evidence)
        and component_trust_anchor_ready(report)
    )


def component_trust_anchor_ready(report: dict[str, Any]) -> bool:
    return evidence_is_operator(report.get("production_origin_trust_anchor"))


def evidence_is_operator(value: Any) -> bool:
    if not isinstance(value, dict):
        return False
    return (
        value.get("valid") is True
        and value.get("evidence_origin") == "operator"
        and value.get("synthetic_evidence") is not True
    )


def evidence_has_required_origin_proof(value: Any) -> bool:
    if not evidence_is_operator(value):
        return False
    return (
        value.get("production_origin_proof_required") is True
        and value.get("production_origin_proof_valid") is True
    )


def synthetic_evidence_reason(report: dict[str, Any], key: str) -> str | None:
    value = report.get(key)
    if not isinstance(value, dict) or value.get("synthetic_evidence") is not True:
        return None
    reasons = value.get("synthetic_evidence_reasons")
    if isinstance(reasons, list) and reasons:
        return "; ".join(str(reason) for reason in reasons)
    return "synthetic evidence origin"


def origin_proof_reason(value: Any, label: str) -> str | None:
    if not isinstance(value, dict):
        return f"{label} evidence absent"
    if not evidence_is_operator(value):
        return None
    if value.get("production_origin_proof_required") is not True:
        return f"{label} evidence did not require production_origin_proof"
    if value.get("production_origin_proof_valid") is not True:
        return f"{label} evidence production_origin_proof_valid=false"
    return None


def component_trust_anchor_reason(report: dict[str, Any], label: str) -> str | None:
    value = report.get("production_origin_trust_anchor")
    if not isinstance(value, dict):
        return f"{label} production_origin_trust_anchor evidence absent"
    if evidence_is_operator(value):
        return None
    if value.get("valid") is not True:
        return f"{label} production_origin_trust_anchor evidence invalid"
    if value.get("evidence_origin") != "operator":
        return f"{label} production_origin_trust_anchor evidence is not operator-origin"
    if value.get("synthetic_evidence") is True:
        return f"{label} production_origin_trust_anchor evidence is synthetic"
    return f"{label} production_origin_trust_anchor evidence is not production-ready"


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = Path(args.root).resolve()
    report_paths = {
        "accountability_receipt": Path(args.accountability_receipt_report),
        "transparency_slo": Path(args.transparency_slo_report),
        "key_management": Path(args.key_management_report),
        "receipt_kms_hsm_custody": Path(args.receipt_kms_hsm_custody_report),
        "production_evidence_handoff_consistency": Path(
            args.handoff_consistency_report
        ),
        "security_release": Path(args.security_release_report),
        "compliance_boundary": Path(args.compliance_boundary_report),
    }
    reports = {name: read_report(path) for name, path in report_paths.items()}
    preflight_report = None
    if args.production_evidence_preflight_report:
        preflight_path = Path(args.production_evidence_preflight_report)
        report_paths["production_evidence_preflight"] = preflight_path
        preflight_report = read_report(preflight_path)
        reports["production_evidence_preflight"] = preflight_report

    failures = doc_failures(root)
    component_status: dict[str, str] = {}
    for name, report in reports.items():
        status = str(report.get("status"))
        component_status[name] = status
        if not report_passed(report):
            failures.append(f"{name}: report status is not a success value: {status}")

    transparency_ready = report_passed(reports["transparency_slo"])
    handoff_consistency_ready = report_passed(
        reports["production_evidence_handoff_consistency"]
    )
    production_preflight_ready = preflight_ready(preflight_report)
    key_custody_ready = kms_hsm_ready(reports["receipt_kms_hsm_custody"])
    compliance_certification_ready = compliance_ready(reports["compliance_boundary"])
    production_ready = (
        not failures
        and production_preflight_ready
        and transparency_ready
        and key_custody_ready
        and compliance_certification_ready
    )

    blockers = []
    if not production_preflight_ready:
        reason = "production evidence preflight report absent"
        if preflight_report is not None:
            if preflight_report.get("production_origin_proof_required") is not True:
                reason = "production evidence preflight did not require origin proof"
            elif preflight_report.get("production_evidence_ready") is not True:
                reason = "production evidence preflight did not pass"
        blockers.append(
            {
                "id": "production_evidence_preflight",
                "status": "blocked",
                "required_gate": (
                    "receipt-production-evidence-production-preflight-check "
                    "passes with production_origin_proof_required=true"
                ),
                "reason": reason,
            }
        )
    if not key_custody_ready:
        synthetic_reason = synthetic_evidence_reason(
            reports["receipt_kms_hsm_custody"], "operator_evidence"
        )
        proof_reason = origin_proof_reason(
            reports["receipt_kms_hsm_custody"].get("operator_evidence"),
            "KMS/HSM custody",
        )
        trust_anchor_reason = component_trust_anchor_reason(
            reports["receipt_kms_hsm_custody"],
            "KMS/HSM custody",
        )
        blockers.append(
            {
                "id": "kms_hsm_receipt_key_custody",
                "status": "blocked",
                "required_gate": (
                    "receipt-kms-hsm-custody-check reports kms_hsm_custody=true "
                    "with operator-origin evidence"
                ),
                "reason": (
                    synthetic_reason
                    or proof_reason
                    or trust_anchor_reason
                    or "operator KMS/HSM custody evidence absent"
                ),
            }
        )
    if not compliance_certification_ready:
        synthetic_reason = synthetic_evidence_reason(
            reports["compliance_boundary"], "external_certification"
        )
        proof_reason = origin_proof_reason(
            reports["compliance_boundary"].get("external_certification"),
            "compliance certification",
        )
        trust_anchor_reason = component_trust_anchor_reason(
            reports["compliance_boundary"],
            "compliance certification",
        )
        blockers.append(
            {
                "id": "compliance_certification",
                "status": "blocked",
                "required_gate": (
                    "compliance-boundary-check reports valid external "
                    "certification, operator-origin evidence, and "
                    "compliance_immutability=true"
                ),
                "reason": (
                    synthetic_reason
                    or proof_reason
                    or trust_anchor_reason
                    or "operator compliance evidence absent"
                ),
            }
        )

    return {
        "schema_version": "cortexdb.receipt_production_readiness.report.v1",
        "status": "passed" if not failures else "failed",
        "production_ready": production_ready,
        "component_status": component_status,
        "reports": {name: str(path) for name, path in report_paths.items()},
        "readiness": {
            "receipt_contract": report_passed(reports["accountability_receipt"]),
            "transparency_operations_evidence": transparency_ready,
            "local_key_management": report_passed(reports["key_management"]),
            "production_evidence_preflight": production_preflight_ready,
            "receipt_kms_hsm_custody_contract": report_passed(
                reports["receipt_kms_hsm_custody"]
            ),
            "production_evidence_handoff_consistency": handoff_consistency_ready,
            "kms_hsm_receipt_key_custody": key_custody_ready,
            "kms_hsm_operator_evidence": evidence_is_operator(
                reports["receipt_kms_hsm_custody"].get("operator_evidence")
            ),
            "kms_hsm_production_origin_trust_anchor": component_trust_anchor_ready(
                reports["receipt_kms_hsm_custody"]
            ),
            "security_release_boundary": report_passed(reports["security_release"]),
            "compliance_certification": compliance_certification_ready,
            "compliance_operator_evidence": evidence_is_operator(
                reports["compliance_boundary"].get("external_certification")
            ),
            "compliance_production_origin_trust_anchor": component_trust_anchor_ready(
                reports["compliance_boundary"]
            ),
        },
        "blockers": blockers,
        "claim_boundary": (
            "release-readiness inventory only; production-grade public receipt "
            "guarantees are not claimed while blockers remain"
        ),
        "checked_docs": REQUIRED_DOC_MARKERS,
        "failures": failures,
    }


def apply_strict_requirement(report: dict[str, Any]) -> None:
    report["strict_production_ready_required"] = True
    if report.get("production_ready") is True:
        return
    report["status"] = "failed"
    failures = report.setdefault("failures", [])
    if isinstance(failures, list):
        failures.append("production_ready=false")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--accountability-receipt-report", required=True)
    parser.add_argument("--transparency-slo-report", required=True)
    parser.add_argument("--key-management-report", required=True)
    parser.add_argument("--receipt-kms-hsm-custody-report", required=True)
    parser.add_argument("--handoff-consistency-report", required=True)
    parser.add_argument("--security-release-report", required=True)
    parser.add_argument("--compliance-boundary-report", required=True)
    parser.add_argument("--production-evidence-preflight-report")
    parser.add_argument("--report", required=True)
    parser.add_argument(
        "--require-production-ready",
        action="store_true",
        help="fail when blockers keep production_ready=false",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except RuntimeError as error:
        print(f"receipt production readiness check failed: {error}", file=sys.stderr)
        return 1
    if args.require_production_ready:
        apply_strict_requirement(report)

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"receipt production readiness check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
