#!/usr/bin/env python3
"""Validate the FC-8 fail-closed end-to-end aggregate wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SUBGATES: tuple[dict[str, str], ...] = (
    {
        "id": "FC-1",
        "name": "cosine metric soundness",
        "command": "$(MAKE) hnsw-cosine-correctness-check",
    },
    {
        "id": "FC-2",
        "name": "persisted ANN/lexical scope parity",
        "command": "$(MAKE) ann-scope-parity-check",
    },
    {
        "id": "FC-3",
        "name": "sparse scope recall",
        "command": "$(MAKE) ann-sparse-scope-recall-check",
    },
    {
        "id": "FC-4",
        "name": "retrieval incompleteness disclosure",
        "command": "$(MAKE) ann-budget-disclosure-check",
    },
    {
        "id": "FC-5",
        "name": "captured access decision",
        "command": "$(MAKE) context-access-decision-capture-check",
    },
    {
        "id": "FC-6",
        "name": "scope leak benchmark",
        "command": "$(MAKE) scope-leak-bench-check",
    },
    {
        "id": "FC-7",
        "name": "fail-closed invariant model",
        "command": "$(MAKE) fail-closed-invariant-model-check",
    },
    {
        "id": "existing",
        "name": "private scope regression",
        "command": "$(MAKE) context-pack-private-scope-check",
    },
    {
        "id": "existing",
        "name": "engine determinism",
        "command": "$(MAKE) engine-determinism-check",
    },
)

REQUIRED_MAKE_TERMS = [
    "fail-closed-end-to-end-check:",
    'python3 scripts/fail_closed_end_to_end_check.py --root "." --report "$(FAIL_CLOSED_END_TO_END_REPORT)"',
]

REQUIRED_VAR_TERMS = [
    "FAIL_CLOSED_END_TO_END_REPORT ?= target/fail-closed-end-to-end/report.json",
]

REQUIRED_PHONY_TERMS = [
    "fail-closed-end-to-end-check",
]

REQUIRED_BETA_BUNDLE_TERMS = [
    '"name": "fail_closed_end_to_end"',
    '"command": ["make", "fail-closed-end-to-end-check"]',
    '"target/fail-closed-end-to-end/report.json"',
]

REQUIRED_STATUS_TERMS = [
    "FC-8",
    "fail-closed-end-to-end-check",
    "beta release lane",
]


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def add_missing_terms(failures: list[str], label: str, text: str, terms: list[str]) -> None:
    for term in terms:
        if not contains_marker(text, term):
            failures.append(f"{label}: missing fail-closed end-to-end marker: {term}")


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, Any] = {
        "subgates": list(SUBGATES),
        "make_terms": REQUIRED_MAKE_TERMS,
        "var_terms": REQUIRED_VAR_TERMS,
        "phony_terms": REQUIRED_PHONY_TERMS,
        "beta_bundle_terms": REQUIRED_BETA_BUNDLE_TERMS,
        "status_terms": REQUIRED_STATUS_TERMS,
    }

    files = {
        "mk/core-retrieval-context.mk": REQUIRED_MAKE_TERMS
        + [subgate["command"] for subgate in SUBGATES],
        "mk/vars-core.mk": REQUIRED_VAR_TERMS,
        "mk/phony.mk": REQUIRED_PHONY_TERMS,
        "scripts/beta_release_bundle.py": REQUIRED_BETA_BUNDLE_TERMS,
        "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": REQUIRED_STATUS_TERMS,
    }
    for rel, terms in files.items():
        try:
            text = read_text(root, rel)
        except FileNotFoundError:
            failures.append(f"{rel}: missing file")
            continue
        add_missing_terms(failures, rel, text, terms)

    return {
        "schema_version": "cortexdb.fail_closed_end_to_end.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "release_blocker": True,
        "beta_release_lane": not any("scripts/beta_release_bundle.py:" in failure for failure in failures),
        "checked": checked,
        "invariant": (
            "FC-1 through FC-7 plus private-scope and determinism gates are "
            "aggregated as one beta release blocker"
        ),
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
    print(f"fail-closed end-to-end check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
