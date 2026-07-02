#!/usr/bin/env python3
"""Validate accountability_receipt.v1 schema freeze wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator


REQUIRED_RECEIPT_FIELDS = [
    "schema_version",
    "header",
    "leaves",
]

REQUIRED_HEADER_FIELDS = [
    "schema_version",
    "hash_alg",
    "sig_alg",
    "db_instance_id",
    "key_id",
    "created_unix_seconds",
    "access_root",
    "provenance_root",
    "cell_set_root",
    "verification_root",
    "budget_commitment",
    "conflict_commitment",
    "pack_root",
    "determinism_hash",
    "audit_chain_head",
    "signature",
]

REQUIRED_LEAF_SETS = [
    "access",
    "provenance",
    "cell_set",
    "verification",
    "budget",
    "conflict",
]

REQUIRED_DOC_TERMS = [
    "docs/schemas/accountability_receipt.v1.json",
    "accountability-receipt-schema-check",
    "accountability_receipt.v1",
    "additive optional field",
    "Runtime JSON receipt emission is fail-closed behind configured receipt key custody",
]

REQUIRED_SDK_TERMS = [
    "pub accountability_receipt: Option<serde_json::Value>",
    "context_pack_v1_deserializes_optional_accountability_receipt",
    "accountability_receipt.v1",
]

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_RECEIPT_SCHEMA_REPORT ?= target/accountability-receipt/schema-report.json",
    "accountability-receipt-schema-check:",
    "cargo test -p cortexdb-sdk context_pack_v1_deserializes_optional_accountability_receipt",
    'python3 scripts/accountability_receipt_schema_check.py --root "." --report "$(ACCOUNTABILITY_RECEIPT_SCHEMA_REPORT)"',
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(read_text(path))


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def schema_errors(schema: dict[str, Any], value: dict[str, Any]) -> list[str]:
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(value), key=lambda err: list(err.path))
    out = []
    for error in errors:
        location = ".".join(str(part) for part in error.path) or "<root>"
        out.append(f"golden schema validation failed at {location}: {error.message}")
    return out


def validate(root: Path) -> dict[str, Any]:
    schema_path = root / "docs/schemas/accountability_receipt.v1.json"
    golden_path = root / "docs/schemas/accountability_receipt.v1.golden.json"
    context_schema_path = root / "docs/schemas/context_pack.v1.json"
    openapi_path = root / "docs/openapi.yaml"

    schema = read_json(schema_path)
    golden = read_json(golden_path)
    context_schema = read_json(context_schema_path)
    openapi = yaml.safe_load(read_text(openapi_path))

    failures: list[str] = []
    failures.extend(schema_errors(schema, golden))

    if schema.get("$id") != "https://cortexdb.local/schemas/accountability_receipt.v1.json":
        failures.append("receipt schema $id must be stable accountability_receipt.v1 URL")
    if schema.get("required") != REQUIRED_RECEIPT_FIELDS:
        failures.append("receipt schema required fields changed")
    header = schema["$defs"]["receipt_header"]
    if header.get("required") != REQUIRED_HEADER_FIELDS:
        failures.append("receipt header required fields changed")
    leaves = schema["$defs"]["receipt_leaves"]
    if leaves.get("required") != REQUIRED_LEAF_SETS:
        failures.append("receipt leaf sets changed")
    if golden["schema_version"] != "accountability_receipt.v1":
        failures.append("golden schema_version must be accountability_receipt.v1")
    if golden["header"]["hash_alg"] != "blake3-256":
        failures.append("golden hash_alg must be blake3-256")
    if golden["header"]["sig_alg"] != "ed25519":
        failures.append("golden sig_alg must be ed25519")

    context_props = context_schema.get("properties", {})
    if "accountability_receipt" not in context_props:
        failures.append("context_pack.v1 schema must expose optional accountability_receipt")

    context_fixture = {
        "schema_version": "context_pack.v1",
        "token_budget_tokens": 1000,
        "estimated_tokens": 42,
        "truncated": False,
        "citations_required": False,
        "answerability_q16": 65535,
        "conflict_visibility_q16": 0,
        "visible_conflict_count": 0,
        "cells": [],
        "anomalies": [],
        "accountability_receipt": golden,
    }
    context_errors = sorted(
        Draft202012Validator(context_schema).iter_errors(context_fixture),
        key=lambda err: list(err.path),
    )
    for error in context_errors:
        location = ".".join(str(part) for part in error.path) or "<root>"
        failures.append(f"context schema receipt validation failed at {location}: {error.message}")

    openapi_context = openapi["components"]["schemas"]["ContextPackResponse"]["properties"]
    receipt_openapi = openapi_context.get("accountability_receipt")
    if receipt_openapi != {"type": "object", "additionalProperties": True, "nullable": True}:
        failures.append("OpenAPI ContextPackResponse must expose nullable accountability_receipt")
    openapi_verification = openapi["components"]["schemas"]["VerificationReportResponse"][
        "properties"
    ]
    verify_receipt_openapi = openapi_verification.get("accountability_receipt")
    if verify_receipt_openapi != {
        "type": "object",
        "additionalProperties": True,
        "nullable": True,
    }:
        failures.append(
            "OpenAPI VerificationReportResponse must expose nullable accountability_receipt"
        )

    docs = "\n".join(
        read_text(root / path)
        for path in [
            "docs/CONTEXT_PACK.md",
            "docs/API_JSON_SCHEMAS.md",
            "docs/spec/ACCOUNTABILITY_RECEIPT_V1.md",
        ]
    )
    sdk = "\n".join(
        read_text(root / path)
        for path in [
            "crates/cortex-sdk/src/types/context.rs",
            "crates/cortex-sdk/src/context_pack_tests.rs",
            "crates/cortex-api-types/src/verification.rs",
        ]
    )
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")

    failures.extend(missing_terms("receipt docs", docs, REQUIRED_DOC_TERMS))
    failures.extend(missing_terms("Rust SDK", sdk, REQUIRED_SDK_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(
        missing_terms("mk/phony.mk", phony, ["accountability-receipt-schema-check"])
    )

    return {
        "schema_version": "cortexdb.accountability_receipt_schema.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "receipt_required_fields": REQUIRED_RECEIPT_FIELDS,
            "header_required_fields": REQUIRED_HEADER_FIELDS,
            "leaf_sets": REQUIRED_LEAF_SETS,
            "doc_terms": REQUIRED_DOC_TERMS,
            "sdk_terms": REQUIRED_SDK_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
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
    print(f"accountability receipt schema check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
