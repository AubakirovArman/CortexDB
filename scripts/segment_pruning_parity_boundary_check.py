#!/usr/bin/env python3
"""Validate the AR-6 vs physical segment-pruning parity boundary."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_TERMS = {
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "physical skipped-segment parity boundary",
        "`ann-scope-parity-check`",
        "Phase 5",
        "not claimed by `context-access-decision-capture-check`",
    ],
    "docs/ACCOUNTABILITY_ROADMAP.md": [
        "`FC-2`",
        "persisted ANN/lexical allowed-set",
        "BoundRetrievePlan's bitmap_program",
        "`make ann-scope-parity-check",
    ],
    "mk/core-retrieval-context.mk": [
        "segment-pruning-parity-boundary-check:",
        'python3 scripts/segment_pruning_parity_boundary_check.py --root "." --report "$(SEGMENT_PRUNING_PARITY_BOUNDARY_REPORT)"',
    ],
    "mk/vars-core.mk": [
        "SEGMENT_PRUNING_PARITY_BOUNDARY_REPORT ?= target/segment-pruning-parity-boundary/report.json",
    ],
    "mk/phony.mk": [
        "segment-pruning-parity-boundary-check",
    ],
}

FORBIDDEN_TERMS = {
    "scripts/context_access_decision_capture_check.py": [
        "ann-scope-parity-check",
        "segment-pruning parity",
        "physical skipped-segment parity",
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
    checked: dict[str, dict[str, list[str]]] = {}
    for rel, terms in REQUIRED_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["required"] = terms
        for term in terms:
            if not contains_marker(text, term):
                failures.append(f"{rel}: missing segment-pruning boundary marker: {term}")
    for rel, terms in FORBIDDEN_TERMS.items():
        text = read_text(root, rel)
        checked.setdefault(rel, {})["forbidden"] = terms
        for term in terms:
            if contains_marker(text, term):
                failures.append(f"{rel}: AR-6 gate claims physical parity marker: {term}")
    return {
        "schema_version": "cortexdb.segment_pruning_parity_boundary.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": checked,
        "boundary": {
            "proves": "AR-6 captured access evidence is documented and gated separately from Phase 5 physical persisted ANN/lexical segment-pruning parity",
            "does_not_prove": "persisted ANN/lexical allowed-set parity with a bound bitmap program",
            "deferred_gate": "ann-scope-parity-check",
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
    print(f"segment-pruning parity boundary check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
