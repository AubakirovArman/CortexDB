#!/usr/bin/env python3
"""Run the local Beta Foundation evidence matrix.

This is a lightweight aggregation gate for the Epic 2 backlog. It does not
replace the heavier production evidence sweep; it proves that the developer-
facing beta foundations are wired and reproducible from one command.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "sdk_contract",
        "command": ["make", "sdk-contract-check"],
        "covers": ["Rust SDK", "Python SDK", "TypeScript SDK", "live local server"],
    },
    {
        "name": "openapi_contract",
        "command": ["make", "openapi-contract-check"],
        "covers": ["OpenAPI", "typed HTTP schema compatibility"],
    },
    {
        "name": "context_verify_quality",
        "command": ["make", "context-verify-quality-check"],
        "covers": ["ContextPack quality fixture", "VERIFY FACT quality fixture"],
    },
    {
        "name": "search_quality",
        "command": ["cargo", "test", "-p", "cortex-engine", "--test", "search_quality"],
        "covers": ["BM25 fixture", "field weights", "multilingual analyzer"],
    },
    {
        "name": "error_taxonomy",
        "command": ["cargo", "test", "-p", "cortex-server", "error_taxonomy"],
        "covers": ["stable error codes", "stable HTTP statuses"],
    },
    {
        "name": "metrics_contract",
        "command": ["cargo", "test", "-p", "cortex-server", "metrics"],
        "covers": ["metrics response shape", "ANN metrics response shape"],
    },
    {
        "name": "beta_delta",
        "command": ["make", "beta-delta-check"],
        "covers": ["beta boundary docs", "release gate wiring"],
    },
)


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def git_sha(repo: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return "unknown"
    return result.stdout.strip()


def run_suite(repo: Path, root: Path, suite: dict[str, Any]) -> dict[str, Any]:
    log_path = root / f"{suite['name']}.log"
    started_at = utc_now()
    result = subprocess.run(
        suite["command"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    output = result.stdout
    if result.stderr:
        output += "\n--- stderr ---\n" + result.stderr
    log_path.write_text(output, encoding="utf-8")
    finished_at = utc_now()
    return {
        "name": suite["name"],
        "status": "passed" if result.returncode == 0 else "failed",
        "exit_code": result.returncode,
        "started_at": started_at,
        "finished_at": finished_at,
        "command": suite["command"],
        "log": str(log_path),
        "covers": suite["covers"],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("beta foundation self-test failed: duplicate suite names")
        return 1
    required = {"sdk_contract", "openapi_contract", "context_verify_quality", "search_quality"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"beta foundation self-test failed: missing suites {missing}")
        return 1
    print("beta foundation self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/beta-foundation")
    parser.add_argument("--report", default="target/beta-foundation/report.json")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    repo = repo_root()
    root = repo / args.root
    report_path = repo / args.report
    root.mkdir(parents=True, exist_ok=True)

    started_at = utc_now()
    suites = [run_suite(repo, root, suite) for suite in SUITES]
    status = "passed" if all(suite["status"] == "passed" for suite in suites) else "failed"
    report = {
        "schema_version": 1,
        "status": status,
        "git_sha": git_sha(repo),
        "started_at": started_at,
        "finished_at": utc_now(),
        "suites": suites,
        "artifacts": {
            "report": str(report_path),
            "root": str(root),
            "production_evidence_sweep": "target/production-evidence/report.json",
        },
        "boundary": {
            "proves": [
                "SDK e2e contract works against a local server",
                "OpenAPI contract is current",
                "ContextPack and VERIFY deterministic quality fixtures pass",
                "Search quality fixtures pass",
                "Error taxonomy and metrics contracts are covered",
            ],
            "does_not_prove": [
                "large-scale beta traffic SLO history",
                "production distributed consensus readiness",
                "unrestricted production HNSW without fallback",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"beta foundation check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"beta foundation check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
