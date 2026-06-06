#!/usr/bin/env python3
"""Validate the cortex-engine error model contract."""

from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path
from typing import Any


FIXTURE = "fixtures/engine/error_model_v1.json"
DOC = "docs/ENGINE_ERROR_MODEL.md"
ERROR_RS = "crates/cortex-engine/src/error.rs"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/engine-error-model/report.json")
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read(relative: str) -> str:
    return (repo_root() / relative).read_text(encoding="utf-8")


def read_json(relative: str) -> dict[str, Any]:
    value = json.loads(read(relative))
    if not isinstance(value, dict):
        raise ValueError(f"{relative} must contain a JSON object")
    return value


def parse_engine_error_variants() -> list[str]:
    text = read(ERROR_RS)
    match = re.search(r"pub enum EngineError \{(?P<body>.*?)\n\}", text, re.S)
    if not match:
        return []
    variants = []
    for line in match.group("body").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#["):
            continue
        name_match = re.match(r"([A-Z][A-Za-z0-9_]+)", stripped)
        if name_match:
            variants.append(name_match.group(1))
    return variants


def pascal_case(code: str) -> str:
    return "".join(part.capitalize() for part in code.split("_"))


def check_fixture(fixture: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if fixture.get("schema_version") != "cortexdb.engine_error_model.v1":
        errors.append("fixture schema_version must be cortexdb.engine_error_model.v1")
    codes = fixture.get("codes", [])
    variants = fixture.get("variants", [])
    if not isinstance(codes, list) or not isinstance(variants, list):
        return ["codes and variants must be lists"]

    code_names = {item.get("code") for item in codes if isinstance(item, dict)}
    variant_names = {item.get("variant") for item in variants if isinstance(item, dict)}
    rust_variants = set(parse_engine_error_variants())
    if variant_names != rust_variants:
        errors.append(
            "EngineError variant fixture mismatch: "
            f"missing={sorted(rust_variants - variant_names)} "
            f"extra={sorted(variant_names - rust_variants)}"
        )

    error_rs = read(ERROR_RS)
    for term in (
        "pub enum EngineErrorCode",
        "pub enum EngineErrorCategory",
        "pub fn code(&self)",
        "pub fn safe_message(&self)",
        "pub fn cli_hint(&self)",
    ):
        if term not in error_rs:
            errors.append(f"{ERROR_RS}: missing {term}")

    doc = read(DOC)
    for code in sorted(code_names):
        if code not in doc:
            errors.append(f"{DOC}: missing code {code}")
        if pascal_case(str(code)) not in read("crates/cortex-sdk/src/types.rs"):
            errors.append(f"crates/cortex-sdk/src/types.rs: missing ErrorCode::{pascal_case(str(code))}")
    for variant in sorted(variant_names):
        if variant not in doc:
            errors.append(f"{DOC}: missing variant {variant}")

    cli = read("crates/cortex-cli/src/cli_ops.rs")
    if "cli_hint()" not in cli:
        errors.append("crates/cortex-cli/src/cli_ops.rs: fmt_engine_error must use cli_hint()")

    server = read("crates/cortex-server/src/responses.rs")
    for term in ("e.code()", "e.safe_message()", "EngineErrorCode::StorageCorruption"):
        if term not in server:
            errors.append(f"crates/cortex-server/src/responses.rs: missing {term}")

    tests = read("crates/cortex-engine/tests/error_model.rs")
    for term in ("EngineErrorCode", "EngineErrorCategory", "safe_message", "cli_hint"):
        if term not in tests:
            errors.append(f"crates/cortex-engine/tests/error_model.rs: missing {term}")

    makefile = read("Makefile")
    for gate in fixture.get("required_evidence_gates", []):
        target = str(gate).removeprefix("make ")
        if f"{target}:" not in makefile:
            errors.append(f"Makefile: missing gate {target}")
    return errors


def main() -> int:
    args = parse_args()
    fixture = read_json(FIXTURE)
    errors = check_fixture(fixture)
    report = {
        "schema_version": "cortexdb.engine_error_model.report.v1",
        "status": "passed" if not errors else "failed",
        "generated_unix_ms": int(time.time() * 1000),
        "fixture": FIXTURE,
        "engine_error_variants": parse_engine_error_variants(),
        "codes": [item.get("code") for item in fixture.get("codes", [])],
        "errors": errors,
    }
    output = repo_root() / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if errors:
        print("engine error model check failed: " + "; ".join(errors))
        return 1
    print(f"engine error model check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
