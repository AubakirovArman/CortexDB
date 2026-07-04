#!/usr/bin/env python3
"""Validate Phase 0 canonical serialization evidence."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


REQUIRED_CANONICAL_TERMS = [
    "CONTEXT_PACK_HASHED_FIELDS",
    "VERIFICATION_REPORT_HASHED_FIELDS",
    "CONTEXT_PACK_EXPORTED_ONLY_FIELDS",
    "VERIFICATION_REPORT_EXPORTED_ONLY_FIELDS",
    "EXCLUDED_TELEMETRY_FIELDS",
    "canonical_context_pack_bytes",
    "canonical_verification_report_bytes",
    "canonical_json_bytes",
    "context_pack.canonical.v1",
    "verification_report.canonical.v1",
]

REQUIRED_TEST_TERMS = [
    "canonical_json_bytes_sort_object_keys_recursively",
    "context_pack_canonical_bytes_are_stable_and_clock_free",
    "verification_report_canonical_bytes_are_stable_and_clock_free",
    "canonical_field_allowlists_are_explicit",
    "canonical_bytes_match_across_processes",
]

REQUIRED_MAKE_TERMS = [
    "canonical-serialization-check:",
    "accountability-canonical-check:",
]

FORBIDDEN_CANONICAL_TERMS = [
    "std::time",
    "Instant::now",
    "SystemTime::now",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden {term}" for term in terms if term in text]


def rust_string_const_values(text: str, const_name: str) -> list[str]:
    pattern = re.compile(
        rf"pub const {re.escape(const_name)}:\s*&\[&str\]\s*=\s*&\[(?P<body>.*?)\];",
        re.S,
    )
    match = pattern.search(text)
    if not match:
        return []
    return re.findall(r'"([^"]+)"', match.group("body"))


def rust_struct_fields(text: str, struct_name: str) -> list[str]:
    pattern = re.compile(
        rf"pub struct {re.escape(struct_name)}\s*\{{(?P<body>.*?)\n\}}",
        re.S,
    )
    match = pattern.search(text)
    if not match:
        return []
    return re.findall(r"^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", match.group("body"), re.M)


def validate_field_classification(root: Path, canonical: str) -> tuple[list[str], list[dict[str, Any]]]:
    checks = [
        {
            "struct": "ContextPack",
            "path": root / "crates/cortex-engine/src/context/mod.rs",
            "hashed_const": "CONTEXT_PACK_HASHED_FIELDS",
            "exported_only_const": "CONTEXT_PACK_EXPORTED_ONLY_FIELDS",
        },
        {
            "struct": "VerificationReport",
            "path": root / "crates/cortex-engine/src/verification/types.rs",
            "hashed_const": "VERIFICATION_REPORT_HASHED_FIELDS",
            "exported_only_const": "VERIFICATION_REPORT_EXPORTED_ONLY_FIELDS",
        },
    ]

    failures: list[str] = []
    summaries: list[dict[str, Any]] = []
    telemetry_fields = set(rust_string_const_values(canonical, "EXCLUDED_TELEMETRY_FIELDS"))

    for check in checks:
        source = read_text(check["path"])
        struct_fields = rust_struct_fields(source, check["struct"])
        hashed_fields = set(rust_string_const_values(canonical, check["hashed_const"]))
        exported_only_fields = set(rust_string_const_values(canonical, check["exported_only_const"]))
        classified_fields = (hashed_fields - {"schema_version"}) | exported_only_fields | telemetry_fields
        unclassified_fields = sorted(set(struct_fields) - classified_fields)

        if not struct_fields:
            failures.append(f"{check['struct']}: struct fields not found")
        if "schema_version" not in hashed_fields:
            failures.append(f"{check['hashed_const']}: missing schema_version")
        for field in unclassified_fields:
            failures.append(f"{check['struct']}: unclassified field {field}")

        summaries.append(
            {
                "struct": check["struct"],
                "struct_fields": sorted(struct_fields),
                "hashed_fields": sorted(hashed_fields),
                "exported_only_fields": sorted(exported_only_fields),
                "telemetry_fields": sorted(telemetry_fields),
                "unclassified_fields": unclassified_fields,
            }
        )

    return failures, summaries


def validate(root: Path) -> dict[str, Any]:
    # canonical.rs was split into canonical/{mod,tests}.rs; the unit-test markers
    # live in tests.rs, so read both for the marker checks below.
    canonical = read_text(root / "crates/cortex-engine/src/canonical/mod.rs") + "\n" + read_text(
        root / "crates/cortex-engine/src/canonical/tests.rs"
    )
    canonical_tests = read_text(root / "crates/cortex-engine/tests/canonical_serialization.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    field_failures, field_summaries = validate_field_classification(root, canonical)

    failures: list[str] = []
    failures.extend(missing_terms("canonical.rs", canonical, REQUIRED_CANONICAL_TERMS))
    failures.extend(
        missing_terms("canonical tests", canonical + "\n" + canonical_tests, REQUIRED_TEST_TERMS)
    )
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(forbidden_terms("canonical.rs", canonical, FORBIDDEN_CANONICAL_TERMS))
    failures.extend(field_failures)

    return {
        "schema_version": "cortexdb.canonical_serialization.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "canonical_terms": REQUIRED_CANONICAL_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_terms": FORBIDDEN_CANONICAL_TERMS,
        },
        "field_classification": field_summaries,
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
    print(f"canonical serialization check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
