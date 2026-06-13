#!/usr/bin/env python3
"""Guard SDK type surfaces against OpenAPI component drift.

This is intentionally a lightweight codegen-control gate: OpenAPI remains the
schema source, while the Rust shared API crate plus Python/TypeScript SDK model
surfaces must expose the same public response fields for migrated components.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

import yaml


REPO = Path(__file__).resolve().parent.parent


@dataclass(frozen=True)
class ComponentContract:
    openapi_name: str
    rust_name: str
    module: str
    python_name: str | None = None
    typescript_name: str | None = None

    @property
    def py_name(self) -> str:
        return self.python_name or self.rust_name

    @property
    def ts_name(self) -> str:
        return self.typescript_name or self.rust_name


COMPONENTS = [
    ComponentContract("HealthResponse", "HealthResponse", "core"),
    ComponentContract("StatsResponse", "StatsResponse", "core"),
    ComponentContract("ValidationResponse", "ValidationResponse", "core"),
    ComponentContract("Cell", "CellResponse", "core"),
    ComponentContract("CellLookupResponse", "CellLookupResponse", "core"),
    ComponentContract("PutCellResponse", "PutCellResponse", "core"),
    ComponentContract("RememberResponse", "RememberResponse", "core"),
    ComponentContract("SearchRoutingDecision", "SearchRoutingDecisionResponse", "search", "SearchRoutingDecision", "SearchRoutingDecision"),
    ComponentContract("AnnNoFallbackDecision", "AnnNoFallbackDecisionResponse", "search", "AnnNoFallbackDecision", "AnnNoFallbackDecision"),
    ComponentContract("AnnSearchReport", "AnnSearchReportResponse", "search", "AnnSearchReport", "AnnSearchReport"),
    ComponentContract("SearchResponse", "SearchResponse", "search"),
    ComponentContract("AnnEvaluationResponse", "AnnEvaluationResponse", "search"),
    ComponentContract("VerificationEvidenceResponse", "EvidenceResponse", "verification"),
    ComponentContract("GuardResponse", "GuardResponse", "verification"),
    ComponentContract("NumericConflictResponse", "NumericConflictResponse", "verification"),
    ComponentContract("VerificationReportResponse", "VerificationReportResponse", "verification"),
    ComponentContract("IngestResponse", "IngestResponse", "ingestion"),
    ComponentContract("IngestionProgress", "IngestionProgress", "ingestion_progress", "IngestionJobResponse", "IngestionJobResponse"),
]


def read(relative: str) -> str:
    return (REPO / relative).read_text(encoding="utf-8")


def openapi_properties(spec: dict, name: str) -> set[str]:
    schema = spec["components"]["schemas"].get(name)
    if schema is None:
        raise KeyError(f"OpenAPI component {name!r} is missing")
    if "properties" not in schema:
        raise KeyError(f"OpenAPI component {name!r} has no object properties")
    return set(schema["properties"])


def rust_fields(text: str, name: str) -> set[str]:
    match = re.search(rf"pub struct {re.escape(name)}\s*\{{(?P<body>.*?)\n\}}", text, re.S)
    if not match:
        return set()
    return set(re.findall(r"\bpub\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:", match.group("body")))


def python_fields(text: str, name: str) -> set[str]:
    match = re.search(rf"^class {re.escape(name)}:\n(?P<body>.*?)(?=^class |\Z)", text, re.S | re.M)
    if not match:
        return set()
    return set(re.findall(r"^    ([a-zA-Z_][a-zA-Z0-9_]*)\s*:", match.group("body"), re.M))


def typescript_fields(text: str, name: str) -> set[str]:
    match = re.search(rf"export interface {re.escape(name)}\s*\{{(?P<body>.*?)\n\}}", text, re.S)
    if not match:
        return set()
    return set(re.findall(r"^\s*([a-zA-Z_][a-zA-Z0-9_]*)\??\s*:", match.group("body"), re.M))


def combined(paths: list[str]) -> str:
    return "\n".join(read(path) for path in paths)


def validate() -> list[str]:
    spec = yaml.safe_load(read("docs/openapi.yaml"))
    failures: list[str] = []

    rust_sources = {
        "core": read("crates/cortex-api-types/src/core.rs"),
        "search": read("crates/cortex-api-types/src/search.rs"),
        "verification": read("crates/cortex-api-types/src/verification.rs"),
        "ingestion": read("crates/cortex-server/src/responses/ingest.rs"),
        "ingestion_progress": read("crates/cortex-engine/src/ingestion/progress.rs"),
    }
    python = combined(
        [
            "sdk/python/_cortexdb_client/model_types/core.py",
            "sdk/python/_cortexdb_client/model_types/search.py",
            "sdk/python/_cortexdb_client/model_types/verification.py",
            "sdk/python/_cortexdb_client/model_types/ingestion.py",
            "sdk/python/_cortexdb_client/model_types/memory.py",
        ]
    )
    ts_sources = [
        "sdk/typescript/cortexdb-client/types/core.ts",
        "sdk/typescript/cortexdb-client/types/search.ts",
        "sdk/typescript/cortexdb-client/types/verification.ts",
        "sdk/typescript/cortexdb-client/types/ingestion.ts",
        "sdk/typescript/cortexdb-client/types/memory.ts",
    ]
    ts = combined(ts_sources)

    for component in COMPONENTS:
        expected = openapi_properties(spec, component.openapi_name)

        rust = rust_fields(rust_sources[component.module], component.rust_name)
        if not rust:
            failures.append(
                f"Rust API type {component.module}::{component.rust_name} is missing"
            )
        elif missing := sorted(expected - rust):
            failures.append(
                f"Rust API type {component.rust_name} misses OpenAPI fields {missing}"
            )

        py = python_fields(python, component.py_name)
        if not py:
            failures.append(f"Python SDK model {component.py_name} is missing")
        elif missing := sorted(expected - py):
            failures.append(
                f"Python SDK model {component.py_name} misses OpenAPI fields {missing}"
            )

        ts_fields_found = typescript_fields(ts, component.ts_name)
        if not ts_fields_found:
            failures.append(f"TypeScript interface {component.ts_name} is missing")
        elif missing := sorted(expected - ts_fields_found):
            failures.append(
                f"TypeScript interface {component.ts_name} misses OpenAPI fields {missing}"
            )

    root_declaration = read("sdk/typescript/cortexdb-client.d.ts")
    if 'export * from "./cortexdb-client/types";' not in root_declaration:
        failures.append("TypeScript root declaration does not re-export modular SDK types")

    server_responses = combined(
        [
            "crates/cortex-server/src/responses/system.rs",
            "crates/cortex-server/src/responses/aql.rs",
            "crates/cortex-server/src/responses/search.rs",
            "crates/cortex-server/src/responses/verification.rs",
        ]
    )
    sdk_responses = combined(
        [
            "crates/cortex-sdk/src/types/core.rs",
            "crates/cortex-sdk/src/types/aql.rs",
            "crates/cortex-sdk/src/types/search.rs",
            "crates/cortex-sdk/src/types/verification.rs",
        ]
    )
    for module in ("core", "search", "verification"):
        needle = f"cortex_api_types::{module}"
        if needle not in server_responses:
            failures.append(f"server responses do not re-export {needle}")
        if needle not in sdk_responses:
            failures.append(f"Rust SDK types do not re-export {needle}")

    return failures


def main() -> int:
    failures = validate()
    if failures:
        print("openapi SDK codegen control check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print("OK: OpenAPI, shared API types, Python SDK, and TypeScript SDK are aligned.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
