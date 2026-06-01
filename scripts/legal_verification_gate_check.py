#!/usr/bin/env python3
"""Validate local legal-grade verification future-epic evidence gates."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


GATES: dict[str, dict[str, object]] = {
    "dataset": {
        "schema": "cortexdb.legal_verification.dataset_gate.v1",
        "markers": [
            ("docs/LEGAL_VERIFICATION_BOUNDARY.md", "Supported Legal Domain"),
            ("docs/LEGAL_VERIFICATION_BOUNDARY.md", "Dataset Fixture"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make legal-verification-dataset-check"),
            ("Makefile", "legal-verification-dataset-check"),
        ],
        "fixture": "crates/cortex-engine/fixtures/legal_verification_dataset_v1.json",
    },
    "quality": {
        "schema": "cortexdb.legal_verification.quality_gate.v1",
        "markers": [
            ("docs/LEGAL_VERIFICATION_BOUNDARY.md", "Quality Gate Boundary"),
            ("docs/VERIFY_FACT.md", "not legal proof"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make legal-verification-quality-check"),
            ("Makefile", "legal-verification-quality-check"),
        ],
    },
    "citation-policy": {
        "schema": "cortexdb.legal_verification.citation_policy_gate.v1",
        "markers": [
            ("docs/LEGAL_VERIFICATION_BOUNDARY.md", "Citation Policy"),
            ("docs/LEGAL_VERIFICATION_BOUNDARY.md", "Citation Policy Fixture"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make legal-citation-policy-check"),
            ("Makefile", "legal-citation-policy-check"),
        ],
        "fixture": "crates/cortex-engine/fixtures/legal_citation_policy_v1.json",
    },
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{path} is invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path} must be a JSON object")
    return value


def validate_markers(markers: list[tuple[str, str]]) -> list[str]:
    failures: list[str] = []
    for file_name, marker in markers:
        if marker not in read(Path(file_name)):
            failures.append(f"marker {marker!r} missing from {file_name}")
    return failures


def validate_dataset(path: Path) -> list[str]:
    failures: list[str] = []
    value = load_json(path)
    if value.get("schema_version") != "cortexdb.legal_verification.dataset_contract.v1":
        failures.append("dataset fixture has wrong schema_version")
    if not isinstance(value.get("selected_domain"), str) or value["selected_domain"] in {"", "generic"}:
        failures.append("selected_domain must name a specific non-generic domain")
    if not isinstance(value.get("jurisdiction"), str) or not value["jurisdiction"]:
        failures.append("jurisdiction must be non-empty")
    if value.get("expert_review_required") is not True:
        failures.append("expert_review_required must be true")
    if value.get("legal_grade_ready") is not False:
        failures.append("dataset fixture must keep legal_grade_ready=false until expert review")
    cases = value.get("cases")
    if not isinstance(cases, list) or not cases:
        return failures + ["dataset fixture must contain cases"]
    for index, case in enumerate(cases):
        if not isinstance(case, dict):
            failures.append(f"case {index} must be an object")
            continue
        for field in ["case_id", "claim", "expected_status", "source_refs", "output_boundary"]:
            if field not in case:
                failures.append(f"case {index} missing {field}")
        if case.get("reviewer_required") is not True:
            failures.append(f"case {index} must require reviewer")
        source_refs = case.get("source_refs")
        if not isinstance(source_refs, list) or not source_refs:
            failures.append(f"case {index} must contain source_refs")
        if case.get("output_boundary") != "evidence_summary_not_legal_advice":
            failures.append(f"case {index} output_boundary must avoid legal advice")
    return failures


def validate_local_review_boundary() -> list[str]:
    failures: list[str] = []
    source = read(Path("crates/cortex-engine/src/legal.rs"))
    markers = [
        "evaluate_legal_verification_boundary",
        "LegalVerificationPolicy",
        "MissingSourceRefs",
        "MissingReviewerApproval",
        "LegalAdviceOutputNotAllowed",
        "legal_grade_ready: false",
    ]
    for marker in markers:
        if marker not in source:
            failures.append(f"legal.rs missing review-boundary marker {marker!r}")
    return failures


def validate_quality(report_path: Path | None) -> list[str]:
    failures: list[str] = []
    if report_path is None:
        return ["quality gate missing --evidence report path"]
    report = load_json(report_path)
    if report.get("status") != "passed":
        failures.append(f"verification quality report status is {report.get('status')!r}")
    if int(report.get("case_count", 0)) <= 0:
        failures.append("verification quality report must include cases")
    if int(report.get("false_positive_count", 1)) != 0:
        failures.append("verification quality report must have zero false positives")
    if int(report.get("false_negative_count", 1)) != 0:
        failures.append("verification quality report must have zero false negatives")
    if int(report.get("citation_guard_cases", 0)) <= 0:
        failures.append("verification quality report must cover missing citation guards")
    if int(report.get("numeric_guard_cases", 0)) <= 0:
        failures.append("verification quality report must cover numeric guard cases")
    return failures


def validate_citation_policy(path: Path) -> list[str]:
    failures: list[str] = []
    value = load_json(path)
    if value.get("schema_version") != "cortexdb.legal_verification.citation_policy.v1":
        failures.append("citation policy fixture has wrong schema_version")
    required_bools = {
        "requires_source_ref": True,
        "requires_reviewer_approval": True,
        "uncited_model_output_admissible": False,
        "legal_advice_output_allowed": False,
        "refuse_unsupported_conclusions": True,
    }
    for field, expected in required_bools.items():
        if value.get(field) is not expected:
            failures.append(f"citation policy {field} must be {expected}")
    sources = value.get("admissible_source_classes")
    if not isinstance(sources, list) or not sources:
        failures.append("citation policy must include admissible source classes")
    retention = value.get("retention_policy")
    if not isinstance(retention, dict) or retention.get("review_audit_required") is not True:
        failures.append("citation policy retention must require review audit")
    return failures


def validate(gate: str, evidence: Path | None) -> dict[str, Any]:
    spec = GATES[gate]
    failures = validate_markers(spec["markers"])  # type: ignore[arg-type]
    checks = {
        "markers": not failures,
        "dataset_fixture": True,
        "quality_report": True,
        "citation_policy": True,
        "local_review_boundary": True,
    }
    if gate == "dataset":
        dataset_failures = validate_dataset(Path(spec["fixture"]))  # type: ignore[arg-type]
        dataset_failures.extend(validate_local_review_boundary())
        failures.extend(dataset_failures)
        checks["dataset_fixture"] = not dataset_failures
        checks["local_review_boundary"] = not dataset_failures
    elif gate == "quality":
        quality_failures = validate_quality(evidence)
        failures.extend(quality_failures)
        checks["quality_report"] = not quality_failures
    elif gate == "citation-policy":
        policy_failures = validate_citation_policy(Path(spec["fixture"]))  # type: ignore[arg-type]
        policy_failures.extend(validate_local_review_boundary())
        failures.extend(policy_failures)
        checks["citation_policy"] = not policy_failures
        checks["local_review_boundary"] = not policy_failures

    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "legal_verification_ready": False,
        "boundary": "local legal verification prerequisites only; no legal advice or legal-grade certification claim",
        "evidence_report": str(evidence) if evidence else "",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATES))
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    try:
        report = validate(args.gate, args.evidence)
    except RuntimeError as error:
        print(f"legal verification gate check failed: {error}", file=sys.stderr)
        return 1
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"legal verification {args.gate} check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
