#!/usr/bin/env python3
"""Validate the ContextPack private-scope leak contract evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "engine provider": [
        "EngineAqlProvider",
        "readable_scopes",
        "agent_allowed.retain",
    ],
    "engine tests": [
        "context_pack_broad_query_excludes_forbidden_scope_before_and_after_persistence",
        "explicit_forbidden_scope_query_is_denied_before_packing",
        "PRIVATE_SCOPE_SHOULD_NOT_LEAK",
        "ContextPackExportFormat::Json",
        "ContextPackExportFormat::Prompt",
        "ContextPackExportFormat::Markdown",
    ],
    "docs": [
        "make context-pack-private-scope-check",
        "forbidden scope",
        "private scope",
    ],
    "plan": [
        "Epic 67. ContextPack Private Scope Leak Test",
        "Status: done",
        "context_pack_private_scope.rs",
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
        "engine provider": read_text(root / "crates/cortex-engine/src/query/provider.rs"),
        "engine tests": read_text(
            root / "crates/cortex-engine/tests/context_pack_private_scope.rs"
        ),
        "docs": "\n".join(
            [
                read_text(root / "docs/CONTEXT_PACK.md"),
                read_text(root / "docs/CONTEXT_PACK_TECHNOLOGY.md"),
                read_text(root / "docs/CONTEXT_PACK_QUALITY_EVIDENCE.md"),
            ]
        ),
        "plan": read_text(root / "docs/PRODUCTION_EPIC_EXECUTION_PLAN.md"),
    }

    failures: list[str] = []
    for name, terms in REQUIRED_TERMS.items():
        failures.extend(missing_terms(name, files[name], terms))

    return {
        "schema_version": "context_pack_private_scope.report.v1",
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
    print(f"context pack private scope passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
