#!/usr/bin/env python3
"""Validate the frozen ContextPack v1 JSON schema contract."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator


REPO = Path(__file__).resolve().parent.parent
SCHEMA_PATH = REPO / "docs/schemas/context_pack.v1.json"
OPENAPI_PATH = REPO / "docs/openapi.yaml"
SNAPSHOT_PATH = (
    REPO
    / "crates/cortex-server/src/tests/snapshots/"
    "cortex_server__tests__response_snapshot_tests__snapshot_context_pack_response.snap"
)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(read(path))


def load_openapi() -> dict[str, Any]:
    return yaml.safe_load(read(OPENAPI_PATH))


def load_snapshot_json(path: Path) -> dict[str, Any]:
    text = read(path)
    parts = text.split("---", 2)
    if len(parts) != 3:
        raise ValueError(f"{path}: expected insta snapshot metadata followed by JSON")
    return json.loads(parts[2])


def schema_required(schema: dict[str, Any], path: list[str]) -> list[str]:
    node: Any = schema
    for part in path:
        node = node[part]
    required = node.get("required")
    if not isinstance(required, list):
        raise ValueError(f"schema path {'.'.join(path)} has no required list")
    return required


def openapi_required(openapi: dict[str, Any], name: str) -> list[str]:
    required = openapi["components"]["schemas"][name].get("required")
    if not isinstance(required, list):
        raise ValueError(f"OpenAPI schema {name} has no required list")
    return required


def assert_same_set(
    failures: list[str],
    label: str,
    left: list[str],
    right: list[str],
) -> None:
    left_set = set(left)
    right_set = set(right)
    if left_set != right_set:
        failures.append(
            f"{label}: missing={sorted(left_set - right_set)} "
            f"extra={sorted(right_set - left_set)}"
        )


def require_contains(
    failures: list[str],
    label: str,
    text: str,
    needles: list[str],
) -> None:
    for needle in needles:
        if needle not in text:
            failures.append(f"{label}: missing {needle}")


def validate() -> list[str]:
    failures: list[str] = []
    schema = load_json(SCHEMA_PATH)
    openapi = load_openapi()
    snapshot = load_snapshot_json(SNAPSHOT_PATH)

    generated = subprocess.run(
        [sys.executable, "scripts/generate_context_pack_sdk_types.py", "--check"],
        cwd=REPO,
        text=True,
        capture_output=True,
    )
    if generated.returncode != 0:
        failures.append(generated.stderr.strip() or generated.stdout.strip())

    validator = Draft202012Validator(schema)
    schema_errors = sorted(validator.iter_errors(snapshot), key=lambda err: list(err.path))
    for error in schema_errors:
        location = ".".join(str(part) for part in error.path) or "<root>"
        failures.append(f"snapshot schema validation failed at {location}: {error.message}")

    if schema.get("$id") != "https://cortexdb.local/schemas/context_pack.v1.json":
        failures.append("schema $id must be stable context_pack.v1 URL")
    if schema["properties"]["schema_version"].get("const") != "context_pack.v1":
        failures.append("schema_version const must be context_pack.v1")

    assert_same_set(
        failures,
        "ContextPackResponse required fields differ",
        schema_required(schema, []),
        openapi_required(openapi, "ContextPackResponse"),
    )
    assert_same_set(
        failures,
        "ContextPackCellResponse required fields differ",
        schema_required(schema, ["$defs", "context_pack_cell"]),
        openapi_required(openapi, "ContextPackCellResponse"),
    )
    assert_same_set(
        failures,
        "ContextAccessDecisionResponse required fields differ",
        schema_required(schema, ["$defs", "context_access_decision"]),
        openapi_required(openapi, "ContextAccessDecisionResponse"),
    )
    assert_same_set(
        failures,
        "ContextSpanProvenanceResponse required fields differ",
        schema_required(schema, ["$defs", "context_span_provenance"]),
        openapi_required(openapi, "ContextSpanProvenanceResponse"),
    )
    assert_same_set(
        failures,
        "ContextPackExplainResponse required fields differ",
        schema_required(schema, ["$defs", "context_pack_explain"]),
        openapi_required(openapi, "ContextPackExplainResponse"),
    )
    assert_same_set(
        failures,
        "SourceRefResponse required fields differ",
        schema_required(schema, ["$defs", "source_ref"]),
        openapi_required(openapi, "SourceRefResponse"),
    )
    assert_same_set(
        failures,
        "ContextPackAnomalyResponse required fields differ",
        schema_required(schema, ["$defs", "context_pack_anomaly"]),
        openapi_required(openapi, "ContextPackAnomalyResponse"),
    )

    sdk_context_pack = read(REPO / "crates/cortex-sdk/src/context_pack.rs")
    sdk_types = read(REPO / "crates/cortex-sdk/src/types/context.rs")
    sdk_tests = read(REPO / "crates/cortex-sdk/src/context_pack_tests.rs")
    require_contains(
        failures,
        "Rust SDK ContextPack v1 aliases",
        sdk_context_pack,
        [
            "pub use crate::generated::context_pack_v1",
            "pub const SCHEMA_VERSION_V1: &'static str = CONTEXT_PACK_V1_SCHEMA_VERSION;",
        ],
    )
    require_contains(
        failures,
        "Rust SDK ContextPack v1 types",
        sdk_types,
        [
            "pub struct ContextPackResponse",
            "pub struct ContextPackCellResponse",
            "pub struct ContextAccessDecisionResponse",
            "pub struct ContextSpanProvenanceResponse",
            "pub struct ContextPackAnomalyResponse",
        ],
    )
    require_contains(
        failures,
        "Rust SDK ContextPack v1 tests",
        sdk_tests,
        [
            "context_pack_v1_sdk_models_roundtrip_full_shape",
            "context_pack_v1_deserializes_without_optional_provenance",
        ],
    )

    docs = "\n".join(
        [
            read(REPO / "docs/CONTEXT_PACK.md"),
            read(REPO / "docs/API_JSON_SCHEMAS.md"),
        ]
    )
    require_contains(
        failures,
        "ContextPack docs",
        docs,
        [
            "docs/schemas/context_pack.v1.json",
            "context-pack-schema-contract-check",
            "additive-only",
        ],
    )

    return failures


def main() -> int:
    failures = validate()
    if failures:
        print("ContextPack v1 schema contract check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print("OK: ContextPack v1 schema, OpenAPI, snapshot, docs, and SDK are aligned.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
