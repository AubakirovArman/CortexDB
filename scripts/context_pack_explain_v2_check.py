#!/usr/bin/env python3
"""Validate the ContextPack Explain v2 contract evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_ENGINE_EXPLAIN_FIELDS = [
    "why_selected",
    "score_components",
    "source_trust_q16",
    "source_trust_category",
    "source_trust_bonus",
    "source_freshness_q16",
    "source_freshness_category",
    "source_freshness_bonus",
    "redundancy_penalty",
]

REQUIRED_SCORE_COMPONENTS = [
    "base_bm25",
    "source_trust_bonus",
    "source_freshness_bonus",
    "redundancy_penalty",
]

REQUIRED_EXCLUSION_CODES = [
    "RedundantCell",
    "TokenOverload",
]

REQUIRED_OPENAPI_FIELDS = [
    "ContextPackExplainResponse",
    "why_selected",
    "score_components",
    "source_trust_q16",
    "source_trust_category",
    "source_trust_bonus",
    "source_freshness_q16",
    "source_freshness_category",
    "source_freshness_bonus",
    "redundancy_penalty",
    "ContextPackAnomalyResponse",
    "why_excluded",
]

REQUIRED_DOC_TERMS = [
    "why_selected",
    "score_components",
    "source_trust_q16",
    "source_trust_category",
    "source_freshness_q16",
    "source_freshness_category",
    "redundancy_penalty",
    "why_excluded",
    "token_budget_tokens",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def require_contains(name: str, text: str, needles: list[str]) -> list[str]:
    return [f"{name}: missing {needle}" for needle in needles if needle not in text]


def validate(root: Path) -> dict[str, Any]:
    checks: dict[str, Any] = {}
    failures: list[str] = []

    engine_mod = read_text(root / "crates/cortex-engine/src/context/mod.rs")
    engine_pack = read_text(root / "crates/cortex-engine/src/context/pack/builder.rs")
    engine_tests = read_text(root / "crates/cortex-engine/tests/context_pack_explain_v2.rs")
    server_responses = read_text(root / "crates/cortex-server/src/responses/context.rs")
    server_context = read_text(root / "crates/cortex-server/src/context/response.rs")
    openapi = read_text(root / "docs/openapi.yaml")
    docs = "\n".join(
        [
            read_text(root / "docs/archive/CONTEXT_PACK_TECHNOLOGY.md"),
            read_text(root / "docs/archive/CONTEXT_PACK_QUALITY_EVIDENCE.md"),
            read_text(root / "docs/API_JSON_SCHEMAS.md"),
        ]
    )

    failures.extend(
        require_contains(
            "engine ContextExplain",
            engine_mod,
            REQUIRED_ENGINE_EXPLAIN_FIELDS,
        )
    )
    failures.extend(
        require_contains(
            "engine score components",
            engine_pack,
            REQUIRED_SCORE_COMPONENTS,
        )
    )
    failures.extend(
        require_contains(
            "engine exclusion reasons",
            engine_pack,
            ["why_excluded", "reduce_redundancy", "token_budget_tokens"],
        )
    )
    failures.extend(
        require_contains(
            "engine explain tests",
            engine_tests,
            [
                "context_pack_explain_v2_reports_selection_source_trust_and_score_components",
                "context_pack_explain_v2_reports_redundancy_exclusion_reason",
                "context_pack_explain_v2_reports_token_budget_exclusion_reason",
            ],
        )
    )
    failures.extend(
        require_contains(
            "engine exclusion codes",
            engine_tests,
            REQUIRED_EXCLUSION_CODES,
        )
    )
    failures.extend(
        require_contains(
            "server explain responses",
            server_responses,
            REQUIRED_ENGINE_EXPLAIN_FIELDS + ["why_excluded"],
        )
    )
    failures.extend(
        require_contains(
            "server context mapper",
            server_context,
            REQUIRED_ENGINE_EXPLAIN_FIELDS + ["why_excluded"],
        )
    )
    failures.extend(
        require_contains(
            "openapi contract",
            openapi,
            REQUIRED_OPENAPI_FIELDS,
        )
    )
    failures.extend(
        require_contains(
            "ContextPack docs",
            docs,
            REQUIRED_DOC_TERMS,
        )
    )

    checks["engine_selected_explain_fields"] = REQUIRED_ENGINE_EXPLAIN_FIELDS
    checks["engine_score_components"] = REQUIRED_SCORE_COMPONENTS
    checks["engine_exclusion_reasons"] = [
        "why_excluded",
        "reduce_redundancy",
        "token_budget_tokens",
    ]
    checks["openapi_fields"] = REQUIRED_OPENAPI_FIELDS
    checks["doc_terms"] = REQUIRED_DOC_TERMS

    return {
        "schema_version": "context_pack_explain_v2.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checks": checks,
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    report_path = Path(args.report)
    report = validate(root)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"context pack explain v2 passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
