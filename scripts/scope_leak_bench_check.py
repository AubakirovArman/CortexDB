#!/usr/bin/env python3
"""Validate the FC-6 scope-leak benchmark wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "crates/cortex-engine/tests/scope_leak_bench.rs": [
        "scope_leak_bench_scans_all_output_surfaces",
        "collect_json_strings",
        "max_visited_candidates: Some(0)",
        "surface_count >= 200",
        "safe_message",
        "numeric_conflicts",
        "source_ref",
        "why_selected",
        "score_components",
    ],
    "crates/cortex-engine/tests/scope_leak_bench/fixture.rs": [
        "PRIVATE_SCOPE_SHOULD_NOT_LEAK",
        "VerificationReportExportFormat::Markdown",
        "VerificationReportExportFormat::Audit",
        "ContextPackExportFormat::Json",
        "ContextPackExportFormat::Prompt",
        "ContextPackExportFormat::Markdown",
    ],
    "mk/core-retrieval-context.mk": [
        "scope-leak-bench-check:",
        "cargo test -p cortex-engine --test scope_leak_bench --all-features",
        'python3 scripts/scope_leak_bench_check.py --root "." --report "$(SCOPE_LEAK_BENCH_REPORT)"',
        "$(MAKE) scope-leak-bench-check",
    ],
    "mk/vars-core.mk": [
        "SCOPE_LEAK_BENCH_REPORT ?= target/context-pack-quality/scope-leak-bench-report.json",
    ],
    "mk/phony.mk": [
        "scope-leak-bench-check",
    ],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "FC-6",
        "scope-leak-bench-check",
        "VERIFY evidence/numeric conflicts",
        "safe errors",
        ">=200",
    ],
}


def contains_marker(text: str, marker: str) -> bool:
    return marker in text or " ".join(marker.split()) in " ".join(text.split())


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    checked: dict[str, list[str]] = {}
    for rel, terms in REQUIRED_TERMS.items():
        try:
            text = read_text(root, rel)
        except FileNotFoundError:
            failures.append(f"{rel}: missing file")
            checked[rel] = terms
            continue
        checked[rel] = terms
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing scope-leak marker: {term}")
    return {
        "schema_version": "cortexdb.scope_leak_bench.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": checked,
        "invariant": (
            "forbidden-scope sentinel strings are absent from ContextPack exports, "
            "VERIFY report exports and fields, numeric conflicts, anomalies, "
            "source_ref/provenance, explain fields, and EngineError.safe_message"
        ),
        "minimum_matrix_combinations": 200,
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
    print(f"scope-leak bench check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
