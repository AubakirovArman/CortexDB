#!/usr/bin/env python3
"""Validate ContextPack token estimator v2 evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_TERMS = {
    "crates/cortex-engine/src/context/token_estimator.rs": [
        "ContextTokenProfile",
        "CortexApproxV2",
        "OpenAiGpt4o",
        "DeepSeekChat",
        "GoogleGemmaIt",
        "BgeM3",
        "estimate_tokens_for_profile",
        "from_model_name",
    ],
    "crates/cortex-engine/src/context/mod.rs": [
        "token_profile: ContextTokenProfile",
        "estimate_tokens_for_profile",
    ],
    "crates/cortex-engine/tests/context_pack_token_estimator.rs": [
        "token_profiles_are_model_specific_for_multilingual_text",
        "context_pack_uses_selected_token_profile",
        "invalid_utf8_payload_falls_back_without_panicking",
    ],
    "docs/CONTEXT_PACK.md": [
        "ContextTokenProfile",
        "Model-specific token profiles",
        "context-pack-token-estimator-check",
    ],
    "docs/archive/CONTEXT_PACK_TECHNOLOGY.md": [
        "Token Estimator v2",
        "model-specific profile",
    ],
    "docs/archive/CONTEXT_PACK_QUALITY_EVIDENCE.md": [
        "Token Estimator v2 Evidence",
        "context_pack_token_estimator",
    ],
    "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md": [
        "Epic 68. ContextPack Token Estimator v2",
        "Status: done",
    ],
}


def validate(root: Path) -> list[dict[str, object]]:
    results: list[dict[str, object]] = []
    for relative, terms in REQUIRED_TERMS.items():
        path = root / relative
        if not path.exists():
            raise SystemExit(f"missing required file: {relative}")
        text = path.read_text(encoding="utf-8")
        missing = [term for term in terms if term not in text]
        if missing:
            raise SystemExit(f"{relative}: missing terms: {', '.join(missing)}")
        results.append(
            {
                "path": relative,
                "checked_terms": terms,
                "status": "ok",
            }
        )
    return results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    args = parser.parse_args()

    root = Path(args.root)
    report = Path(args.report)
    results = validate(root)
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(
        json.dumps(
            {
                "gate": "context_pack_token_estimator_v2",
                "status": "passed",
                "checks": results,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"context pack token estimator check passed: {report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
