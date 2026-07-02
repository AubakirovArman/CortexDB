#!/usr/bin/env python3
"""Aggregate accountability receipt schema, determinism, sign, verify, and tamper gates."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SUCCESS_STATUSES = {"passed", "ok"}

REPORT_ARGS = {
    "schema": "schema_report",
    "determinism": "determinism_report",
    "sign": "sign_report",
    "verify": "verify_report",
    "tamper": "tamper_report",
}

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_RECEIPT_REPORT ?= target/accountability-receipt/report.json",
    "accountability-receipt-check:",
    "$(MAKE) accountability-receipt-schema-check",
    "$(MAKE) accountability-receipt-determinism-check",
    "$(MAKE) accountability-receipt-sign-check",
    "$(MAKE) accountability-receipt-verify-check",
    "$(MAKE) accountability-receipt-tamper-check",
    'python3 scripts/accountability_receipt_check.py --root "."',
    "$(MAKE) accountability-receipt-check",
]

INTEGRITY_SOURCE_GLOBS = [
    "crates/cortex-engine/src/accountability.rs",
    "crates/cortex-engine/src/accountability/*.rs",
    "crates/cortex-receipt-verify/src/*.rs",
    "crates/cortex-crypto/src/receipt_key.rs",
]

FORBIDDEN_INTEGRITY_TERMS = ["FNV", "fnv", "xor-fnv", "xor_fnv", "DefaultHasher"]


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def report_passed(report: dict[str, Any]) -> bool:
    return report.get("status") in SUCCESS_STATUSES


def production_safe(report: dict[str, Any]) -> bool:
    return report.get("production_safe") is not False


def makefiles_text(root: Path) -> str:
    mk_text = "\n".join(path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk")))
    return mk_text + "\n" + (root / "mk/phony.mk").read_text(encoding="utf-8")


def source_text(root: Path) -> str:
    chunks = []
    for pattern in INTEGRITY_SOURCE_GLOBS:
        for path in sorted(root.glob(pattern)):
            chunks.append(path.read_text(encoding="utf-8"))
    return "\n".join(chunks)


def validate(args: argparse.Namespace) -> dict[str, Any]:
    root = Path(args.root).resolve()
    reports = {
        name: read_json(Path(getattr(args, attr_name)))
        for name, attr_name in REPORT_ARGS.items()
    }
    report_paths = {
        name: str(Path(getattr(args, attr_name))) for name, attr_name in REPORT_ARGS.items()
    }

    failures: list[str] = []
    component_status: dict[str, str] = {}
    for name, report in reports.items():
        status = str(report.get("status"))
        component_status[name] = status
        if not report_passed(report):
            failures.append(f"{name}: report status is not a success value: {status}")
        if not production_safe(report):
            failures.append(f"{name}: production_safe is false")

    make_text = makefiles_text(root)
    for term in REQUIRED_MAKE_TERMS:
        if term not in make_text:
            failures.append(f"make wiring missing {term}")

    integrity_text = source_text(root)
    for term in FORBIDDEN_INTEGRITY_TERMS:
        if term in integrity_text:
            failures.append(f"receipt integrity source contains forbidden term {term}")

    return {
        "schema_version": "cortexdb.accountability_receipt.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "component_status": component_status,
        "reports": report_paths,
        "checked": {
            "success_statuses": sorted(SUCCESS_STATUSES),
            "make_terms": REQUIRED_MAKE_TERMS,
            "integrity_source_globs": INTEGRITY_SOURCE_GLOBS,
            "forbidden_integrity_terms": FORBIDDEN_INTEGRITY_TERMS,
        },
        "failures": failures,
        "boundary": {
            "proves": "accountability receipt schema, deterministic body roots, signed header, standalone verifier, and tamper rejection are all gated",
            "does_not_prove": "runtime route coverage, receipt/audit re-anchor records, transparency log anchoring, durable database-instance identity, or physical ANN fail-closed parity",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--schema-report", required=True)
    parser.add_argument("--determinism-report", required=True)
    parser.add_argument("--sign-report", required=True)
    parser.add_argument("--verify-report", required=True)
    parser.add_argument("--tamper-report", required=True)
    parser.add_argument("--report", required=True)
    args = parser.parse_args()

    try:
        report = validate(args)
    except RuntimeError as error:
        print(f"accountability receipt check failed: {error}")
        return 1

    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"accountability receipt check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
