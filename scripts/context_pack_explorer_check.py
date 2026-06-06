#!/usr/bin/env python3
"""Validate ContextPack Explorer dashboard wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "html": [
        ("web/dashboard/src/index.html", "id=\"context-report\""),
        ("web/dashboard/src/index.html", "Build Context Pack"),
        ("web/dashboard/src/index.html", "Context Pack report"),
    ],
    "renderer": [
        ("web/dashboard/src/reporting_retrieval.js", "renderContextPack"),
        ("web/dashboard/src/reporting_retrieval.js", "Context cells"),
        ("web/dashboard/src/reporting_retrieval.js", "Source refs"),
        ("web/dashboard/src/reporting_retrieval.js", "Token budget"),
        ("web/dashboard/src/reporting_retrieval.js", "Estimated tokens"),
        ("web/dashboard/src/reporting_retrieval.js", "Citation explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Explain explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Anomaly explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "score_components"),
        ("web/dashboard/src/reporting_retrieval.js", "why_excluded"),
    ],
    "docs": [
        ("docs/DASHBOARD_UI.md", "ContextPack Explorer"),
        ("docs/DASHBOARD_UI.md", "token budget usage"),
        ("docs/PRODUCTION_EPIC_EXECUTION_PLAN.md", "Epic 112. ContextPack Explorer"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except RuntimeError as error:
        print(f"context pack explorer check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"context pack explorer check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
