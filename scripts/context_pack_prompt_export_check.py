#!/usr/bin/env python3
"""Validate the ContextPack prompt/export contract evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_ENGINE_FORMATS = [
    "ContextPackExportFormat::Json",
    "ContextPackExportFormat::Prompt",
    "ContextPackExportFormat::Markdown",
]

REQUIRED_PROMPT_LINES = [
    "Use only the context cells below.",
    "Preserve citations when answering.",
    "Cite citation= or source_ref= values for factual claims.",
    "If the supplied context is insufficient or conflicting, say so.",
    "Do not resolve conflicting evidence silently",
]

REQUIRED_JSON_FIELDS = [
    "schema_version",
    "token_budget_tokens",
    "estimated_tokens",
    "answerability_q16",
    "conflict_visibility_q16",
    "visible_conflict_count",
    "citations_required",
    "cells",
    "anomalies",
    "source_ref",
    "explain",
]

REQUIRED_TESTS = [
    "context_pack_prompt_export_includes_citation_and_conflict_instructions",
    "context_pack_markdown_export_is_stable_and_cited",
    "context_pack_json_export_has_public_schema_fields",
]

REQUIRED_DOC_TERMS = [
    "format=prompt",
    "format=markdown",
    "typed JSON",
    "Preserve citations when answering.",
    "If the supplied context is insufficient or conflicting, say so.",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def require_contains(name: str, text: str, needles: list[str]) -> list[str]:
    return [f"{name}: missing {needle}" for needle in needles if needle not in text]


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    export_rs = read_text(root / "crates/cortex-engine/src/context/export.rs")
    json_export = read_text(root / "crates/cortex-engine/src/context/export/json_export.rs")
    tests = read_text(root / "crates/cortex-engine/tests/context_pack_prompt_export.rs")
    cli = read_text(root / "crates/cortex-cli/src/cli_ops.rs")
    server = read_text(root / "crates/cortex-server/src/context.rs")
    openapi = read_text(root / "docs/openapi.yaml")
    docs = "\n".join(
        [
            read_text(root / "docs/API.md"),
            read_text(root / "docs/API_JSON_SCHEMAS.md"),
            read_text(root / "docs/archive/CONTEXT_PACK_QUALITY_EVIDENCE.md"),
            read_text(root / "docs/archive/CONTEXT_PACK_TECHNOLOGY.md"),
        ]
    )

    failures.extend(require_contains("engine export formats", export_rs, REQUIRED_ENGINE_FORMATS))
    failures.extend(require_contains("engine prompt instructions", export_rs, REQUIRED_PROMPT_LINES))
    failures.extend(require_contains("engine json export fields", json_export, REQUIRED_JSON_FIELDS))
    failures.extend(require_contains("engine export tests", tests, REQUIRED_TESTS))
    failures.extend(require_contains("cli context formats", cli, ["json", "prompt", "markdown"]))
    failures.extend(require_contains("server context formats", server, ["json", "prompt", "markdown"]))
    failures.extend(require_contains("openapi context formats", openapi, ["format", "prompt", "markdown"]))
    failures.extend(require_contains("docs", docs, REQUIRED_DOC_TERMS))

    return {
        "schema_version": "context_pack_prompt_export.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checks": {
            "engine_formats": REQUIRED_ENGINE_FORMATS,
            "prompt_instructions": REQUIRED_PROMPT_LINES,
            "json_fields": REQUIRED_JSON_FIELDS,
            "tests": REQUIRED_TESTS,
            "public_surfaces": ["engine", "cli", "server", "openapi", "docs"],
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
    print(f"context pack prompt export passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
