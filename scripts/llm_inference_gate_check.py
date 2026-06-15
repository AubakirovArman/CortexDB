#!/usr/bin/env python3
"""Validate local built-in LLM inference future-epic evidence gates."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from llm_inference_runtime_safety import (
    validate_runtime_safety_config,
    validate_runtime_safety_marker,
)


FORBIDDEN_ENDPOINTS = ("/v1/llm", "/v1/chat")
LLM_INFERENCE_DESIGN_DOC = "docs/archive/LLM_INFERENCE_DESIGN.md"

GATES: dict[str, dict[str, object]] = {
    "contract": {
        "schema": "cortexdb.llm_inference.contract_gate.v1",
        "markers": [
            (LLM_INFERENCE_DESIGN_DOC, "API Contract Boundary"),
            ("docs/API_JSON_SCHEMAS.md", "LLM Inference Test-double Endpoint"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make llm-inference-contract-check"),
            ("README.md", "No production built-in LLM runtime"),
            ("Makefile", "llm-inference-contract-check"),
        ],
    },
    "safety": {
        "schema": "cortexdb.llm_inference.safety_gate.v1",
        "markers": [
            (LLM_INFERENCE_DESIGN_DOC, "Prompt Visibility"),
            (LLM_INFERENCE_DESIGN_DOC, "ContextPack Boundary"),
            (LLM_INFERENCE_DESIGN_DOC, "Resource Limits"),
            (LLM_INFERENCE_DESIGN_DOC, "Runtime Safety Config"),
            (LLM_INFERENCE_DESIGN_DOC, "Safety And Audit"),
            ("crates/cortex-server/src/audit.rs", "emit_llm_inference_decision"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make llm-inference-safety-check"),
            ("Makefile", "llm-inference-safety-check"),
        ],
        "fixture": "crates/cortex-server/fixtures/llm_runtime_safety_config_v1.json",
    },
    "smoke": {
        "schema": "cortexdb.llm_inference.smoke_gate.v1",
        "markers": [
            (LLM_INFERENCE_DESIGN_DOC, "Deterministic Test Double"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make llm-inference-smoke-check"),
            ("Makefile", "llm-inference-smoke-check"),
        ],
        "fixtures": [
            "crates/cortex-engine/fixtures/llm_inference_smoke_request_v1.json",
            "crates/cortex-engine/fixtures/llm_inference_smoke_response_v1.json",
        ],
    },
    "secrets": {
        "schema": "cortexdb.llm_inference.secrets_gate.v1",
        "markers": [
            (LLM_INFERENCE_DESIGN_DOC, "Provider keys must come from runtime environment only"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make secrets-check"),
            ("Makefile", "secrets-check"),
        ],
    },
}

SECRET_PATTERNS = (
    re.compile(r"(?i)\bOPENAI_API_KEY\s*=\s*(?!dummy|example|placeholder|<)[A-Za-z0-9_\-]{12,}"),
    re.compile(r"(?i)\bCORTEXDB_[A-Z0-9_]*API_KEY\s*=\s*(?!dummy|example|placeholder|<)[A-Za-z0-9_\-]{12,}"),
    re.compile(r"\bsk-[A-Za-z0-9_\-]{20,}\b"),
)

SCAN_EXCLUDED_PARTS = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
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
        raise RuntimeError(f"failed to read fixture {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"fixture {path} is invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"fixture {path} must be a JSON object")
    return value


def validate_markers(markers: list[tuple[str, str]]) -> list[str]:
    failures: list[str] = []
    for file_name, marker in markers:
        if marker not in read(Path(file_name)):
            failures.append(f"marker {marker!r} missing from {file_name}")
    return failures


def tracked_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError("failed to list tracked files with git")
    files: list[Path] = []
    for line in result.stdout.splitlines():
        path = Path(line)
        if any(part in SCAN_EXCLUDED_PARTS for part in path.parts):
            continue
        files.append(path)
    return files


def validate_no_forbidden_endpoints() -> list[str]:
    failures: list[str] = []
    for relative in [
        Path("docs/openapi.yaml"),
        Path("crates/cortex-server/src/router.rs"),
        Path("crates/cortex-server/src/audit.rs"),
    ]:
        text = read(relative)
        for endpoint in FORBIDDEN_ENDPOINTS:
            if endpoint in text:
                failures.append(f"{relative}: forbidden future inference endpoint {endpoint!r} is exposed")
    return failures


def validate_test_double_endpoint_contract() -> list[str]:
    failures: list[str] = []
    openapi = read(Path("docs/openapi.yaml"))
    server = read(Path("crates/cortex-server/src/llm.rs"))
    main = read(Path("crates/cortex-server/src/main.rs"))
    design = read(Path(LLM_INFERENCE_DESIGN_DOC))
    expected = [
        (openapi, "/v1/inference", "docs/openapi.yaml"),
        (openapi, "LlmInferenceResponse", "docs/openapi.yaml"),
        (server, "handle_inference_test_double", "crates/cortex-server/src/llm.rs"),
        (main, "CORTEXDB_LLM_TEST_DOUBLE", "crates/cortex-server/src/main.rs"),
        (design, "disabled by default", LLM_INFERENCE_DESIGN_DOC),
        (design, "test-double", LLM_INFERENCE_DESIGN_DOC),
    ]
    for text, marker, path in expected:
        if marker not in text:
            failures.append(f"{path}: missing test-double endpoint marker {marker!r}")
    return failures


def validate_smoke_fixtures(paths: list[str]) -> list[str]:
    failures: list[str] = []
    request = load_json(Path(paths[0]))
    response = load_json(Path(paths[1]))
    if request.get("schema_version") != "cortexdb.llm_inference.smoke_request.v1":
        failures.append("smoke request fixture has wrong schema_version")
    if response.get("schema_version") != "cortexdb.llm_inference.smoke_response.v1":
        failures.append("smoke response fixture has wrong schema_version")
    if request.get("provider") != "test_double" or response.get("provider") != "test_double":
        failures.append("smoke fixtures must use deterministic test_double provider")
    if request.get("api_key") not in (None, ""):
        failures.append("smoke request fixture must not contain an api_key")
    context = request.get("context_pack")
    if not isinstance(context, dict) or not context.get("cells"):
        failures.append("smoke request must include non-empty explicit ContextPack cells")
    audit = response.get("audit")
    if not isinstance(audit, dict):
        failures.append("smoke response must include audit object")
    else:
        for field in ["context_pack_only", "prompt_body_logged", "secrets_logged"]:
            if field not in audit:
                failures.append(f"smoke response audit missing {field}")
        if audit.get("context_pack_only") is not True:
            failures.append("smoke response must assert context_pack_only=true")
        if audit.get("prompt_body_logged") is not False:
            failures.append("smoke response must assert prompt_body_logged=false")
        if audit.get("secrets_logged") is not False:
            failures.append("smoke response must assert secrets_logged=false")
    return failures


def validate_tracked_secrets() -> list[str]:
    failures: list[str] = []
    for path in tracked_files():
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        except OSError as error:
            failures.append(f"{path}: failed to read tracked file: {error}")
            continue
        for pattern in SECRET_PATTERNS:
            if pattern.search(text):
                failures.append(f"{path}: tracked provider-secret-like literal detected")
                break
    return failures


def validate(gate: str) -> dict[str, Any]:
    spec = GATES[gate]
    failures = validate_markers(spec["markers"])  # type: ignore[arg-type]
    checks: dict[str, bool] = {
        "markers": not failures,
        "contract_boundary": True,
        "safety_boundary": True,
        "deterministic_smoke_fixture": True,
        "tracked_secrets_absent": True,
    }

    if gate == "contract":
        contract_failures = validate_no_forbidden_endpoints()
        contract_failures.extend(validate_test_double_endpoint_contract())
        failures.extend(contract_failures)
        checks["contract_boundary"] = not contract_failures
    elif gate == "safety":
        text = read(Path(LLM_INFERENCE_DESIGN_DOC))
        required = [
            "cannot bypass AgentView",
            "must not log full prompt bodies by default",
            "request size limits",
            "timeouts",
            "queue backpressure",
            "llm_inference_decision",
        ]
        missing = [item for item in required if item not in text]
        failures.extend(f"{LLM_INFERENCE_DESIGN_DOC} missing safety rule {item!r}" for item in missing)
        runtime_failures = validate_runtime_safety_config(Path(spec["fixture"]))  # type: ignore[arg-type]
        runtime_failures.extend(validate_runtime_safety_marker())
        failures.extend(runtime_failures)
        checks["safety_boundary"] = not missing and not runtime_failures
    elif gate == "smoke":
        fixture_failures = validate_smoke_fixtures(spec["fixtures"])  # type: ignore[arg-type]
        failures.extend(fixture_failures)
        checks["deterministic_smoke_fixture"] = not fixture_failures
    elif gate == "secrets":
        secret_failures = validate_tracked_secrets()
        failures.extend(secret_failures)
        checks["tracked_secrets_absent"] = not secret_failures

    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "built_in_llm_ready": False,
        "boundary": "local built-in LLM inference test-double contract only; no production model runtime claim",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATES))
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    try:
        report = validate(args.gate)
    except RuntimeError as error:
        print(f"LLM inference gate check failed: {error}", file=sys.stderr)
        return 1
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"LLM inference {args.gate} check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
