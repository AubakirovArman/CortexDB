"""Runtime safety checks for the local LLM inference future-epic gate."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def _load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise RuntimeError(f"{path} must be a JSON object")
    return value


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def validate_runtime_safety_config(path: Path) -> list[str]:
    failures: list[str] = []
    value = _load_json(path)
    if value.get("schema_version") != "cortexdb.llm_inference.runtime_safety_config.v1":
        failures.append("runtime safety config has wrong schema_version")
    if value.get("built_in_llm_ready") is not False:
        failures.append("runtime safety config must keep built_in_llm_ready=false")
    positive_fields = [
        "max_prompt_bytes",
        "max_context_cells",
        "max_output_tokens",
        "request_timeout_ms",
        "queue_capacity",
        "max_concurrent_requests",
    ]
    for field in positive_fields:
        if int(value.get(field, 0)) <= 0:
            failures.append(f"runtime safety config {field} must be positive")
    if int(value.get("max_prompt_bytes", 0)) > 256 * 1024:
        failures.append("runtime safety config max_prompt_bytes is too large")
    if value.get("request_api_keys_allowed") is not False:
        failures.append("runtime safety config must reject request body API keys")
    if value.get("prompt_body_logging_enabled") is not False:
        failures.append("runtime safety config must keep prompt body logging disabled")
    return failures


def validate_runtime_safety_marker() -> list[str]:
    source = _read(Path("crates/cortex-server/src/llm/safety.rs"))
    markers = [
        "LlmRuntimeSafetyConfig",
        "validate_llm_runtime_safety_config",
        "RequestApiKeysNotAllowed",
        "PromptBodyLoggingNotAllowed",
        "max_concurrent_requests",
    ]
    return [
        f"llm/safety.rs missing runtime safety marker {marker!r}"
        for marker in markers
        if marker not in source
    ]
