#!/usr/bin/env python3
"""Validate CortexDB's local compliance boundary documentation."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "boundary_schema": [
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb.compliance_boundary.v1"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "Supported certified frameworks today: none."),
    ],
    "non_claims": [
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "SOC 2 compliance"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "ISO 27001 certification"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "HIPAA compliance"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "GDPR processor or controller compliance"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "legal-grade fact verification"),
    ],
    "local_evidence_controls": [
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "CORTEXDB_AUTH_POLICY_STORE_FILE"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb auth-review"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb audit --verify-chain"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "cortexdb audit-export-siem"),
        ("docs/COMPLIANCE_BOUNDARY_MAPPING.md", "make public-claims-check"),
    ],
    "security_docs_boundary": [
        ("docs/SECURITY_HARDENING_EVIDENCE.md", "It does not claim full RBAC"),
        ("docs/SECURITY_BETA_BASELINE.md", "It does not mean CortexDB has external security certification"),
        ("docs/ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md", "must not imply certification"),
        ("docs/FUTURE_NON_GOAL_EPICS.md", "Public docs state exactly which compliance frameworks are supported"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return {
        "schema_version": "cortexdb.compliance_boundary_check.v1",
        "status": "passed" if not failures else "failed",
        "supported_certified_frameworks": [],
        "claim_boundary": "local evidence only; no external compliance certification",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except RuntimeError as error:
        print(f"compliance boundary check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"compliance boundary check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
