#!/usr/bin/env python3
"""Validate the ContextPack answerability score contract evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "engine structs": [
        "answerability_q16",
        "InsufficientContext",
        "insufficient_context",
    ],
    "answerability module": [
        "DEFAULT_ANSWERABILITY_THRESHOLD_Q16",
        "missing_terms",
        "covered_terms",
    ],
    "engine tests": [
        "answerability_is_full_when_selected_cells_cover_query_terms",
        "answerability_reports_missing_query_terms",
        "answerability_reports_empty_context",
        "answerability_is_exported_in_json_prompt_and_markdown",
    ],
    "public json": [
        "answerability_q16",
        "insufficient_context",
    ],
    "docs": [
        "answerability_q16",
        "insufficient_context",
        "make context-pack-answerability-check",
    ],
}


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(name: str, text: str, terms: list[str]) -> list[str]:
    return [f"{name}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    files = {
        "engine structs": read_text(root / "crates/cortex-engine/src/context/mod.rs"),
        "answerability module": read_text(
            root / "crates/cortex-engine/src/context/answerability.rs"
        ),
        "engine tests": read_text(
            root / "crates/cortex-engine/tests/context_pack_answerability.rs"
        ),
        "public json": "\n".join(
            [
                read_text(root / "crates/cortex-engine/src/context/export/json_export.rs"),
                read_text(root / "crates/cortex-server/src/responses.rs"),
                read_text(root / "crates/cortex-cli/src/cli_json_types.rs"),
                read_text(root / "crates/cortex-sdk/src/types.rs"),
                read_text(root / "docs/openapi.yaml"),
            ]
        ),
        "docs": "\n".join(
            [
                read_text(root / "docs/CONTEXT_PACK.md"),
                read_text(root / "docs/CONTEXT_PACK_TECHNOLOGY.md"),
                read_text(root / "docs/CONTEXT_PACK_QUALITY_EVIDENCE.md"),
                read_text(root / "docs/API_JSON_SCHEMAS.md"),
            ]
        ),
    }

    failures: list[str] = []
    for name, terms in REQUIRED_TERMS.items():
        failures.extend(missing_terms(name, files[name], terms))

    return {
        "schema_version": "context_pack_answerability.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checks": REQUIRED_TERMS,
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
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"context pack answerability passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
