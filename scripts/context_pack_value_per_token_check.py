#!/usr/bin/env python3
"""ContextPack value-per-token planner gate for EPIC-F07."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


DOC_PATH = Path("docs/LLM_CONTEXT_VALUE_OPTIMIZATION.md")
REQUIRED_DOC_MARKERS = [
    "optimize_value_per_token",
    "marginal query-term coverage",
    "canonical ContextPack BM25 score",
    "source trust bonus",
    "source freshness bonus",
    "citation availability",
    "decayed feedback bonus",
    "redundancy penalty",
    "deterministic token cost",
    "does not call an LLM",
]
REGRESSION_COMMAND = [
    "cargo",
    "test",
    "-p",
    "cortex-engine",
    "--test",
    "context_pack",
    "--all-features",
    "value_per_token",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    return parser.parse_args()


def check_doc_markers() -> list[str]:
    if not DOC_PATH.exists():
        return [f"{DOC_PATH} is missing"]
    text = DOC_PATH.read_text(encoding="utf-8")
    return [
        f"{DOC_PATH} missing marker: {marker}"
        for marker in REQUIRED_DOC_MARKERS
        if marker not in text
    ]


def run_regression() -> tuple[int, str, str]:
    completed = subprocess.run(
        REGRESSION_COMMAND,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.returncode, completed.stdout, completed.stderr


def main() -> int:
    args = parse_args()
    errors = check_doc_markers()
    returncode, stdout, stderr = run_regression()
    if returncode != 0:
        errors.append("context_pack value_per_token regression test failed")

    report = {
        "schema_version": "cortexdb.context_pack_value_per_token.v1",
        "doc": str(DOC_PATH),
        "required_doc_markers": REQUIRED_DOC_MARKERS,
        "regression_command": REGRESSION_COMMAND,
        "regression_returncode": returncode,
        "status": "passed" if not errors else "failed",
        "errors": errors,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    summary = {
        "status": report["status"],
        "report": args.report,
        "regression_command": " ".join(REGRESSION_COMMAND),
    }
    print(json.dumps(summary, sort_keys=True))
    if stdout:
        print(stdout, end="")
    if stderr:
        print(stderr, end="", file=sys.stderr)
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
