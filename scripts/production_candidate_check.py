#!/usr/bin/env python3
"""Run the local single-node Production Candidate evidence matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "docs/RPO_RTO.md": (
        "RPO",
        "RTO",
        "make backup-drill-check",
        "single-node",
    ),
    "docs/archive/SINGLE_NODE_SLO.md": (
        "SLO",
        "make single-node-performance-check",
        "target/single-node-performance/report.json",
    ),
    "docs/archive/API_COMPATIBILITY.md": (
        "OpenAPI",
        "SDK",
        "compatibility",
    ),
    "docs/archive/SDK_RELEASE.md": (
        "sdk-release-contract-check",
        "deprecation",
        "version",
    ),
    "docs/archive/UPGRADE_MIGRATION.md": (
        "Upgrade Workflow",
        "Rollback Workflow",
        "make migration-compatibility-check",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "production_hardening",
        "command": ["make", "production-hardening-check"],
        "covers": ["load", "crash/fault", "migration", "audit", "rate-limit"],
    },
    {
        "name": "backup_rpo_rto_drill",
        "command": ["make", "backup-rpo-rto-profile-check"],
        "covers": ["RPO/RTO profiles", "backup timing", "restore timing", "data-loss boundary"],
    },
    {
        "name": "single_node_slo",
        "command": ["make", "single-node-performance-check"],
        "covers": ["single-node lifecycle duration", "performance evidence"],
    },
    {
        "name": "openapi_contract",
        "command": ["make", "openapi-contract-check"],
        "covers": ["HTTP API schema compatibility", "typed response contract"],
    },
    {
        "name": "sdk_release_contract",
        "command": ["make", "sdk-release-contract-check"],
        "covers": ["SDK package metadata", "release workflow", "version lock-step"],
    },
    {
        "name": "sdk_deprecation_policy",
        "command": ["make", "sdk-deprecation-check"],
        "covers": ["deprecated routes", "SDK source compatibility", "changelog policy"],
    },
    {
        "name": "migration_policy",
        "command": ["make", "migration-policy-check"],
        "covers": ["upgrade docs", "rollback docs", "format markers"],
    },
    {
        "name": "migration_compatibility",
        "command": ["make", "migration-compatibility-check"],
        "covers": ["upgrade/downgrade fixture", "storage/API/SDK compatibility"],
    },
    {
        "name": "binary_release",
        "command": ["make", "binary-release-check"],
        "covers": ["CLI/server release binaries", "archive validation"],
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
    return {
        "name": suite["name"],
        "status": "passed" if result.returncode == 0 else "failed",
        "exit_code": result.returncode,
        "started_at": started_at,
        "finished_at": utc_now(),
        "command": suite["command"],
        "log": str(log_path),
        "covers": suite["covers"],
    }


def check_docs(repo: Path) -> dict[str, Any]:
    missing: list[str] = []
    for relative, terms in DOC_REQUIREMENTS.items():
        path = repo / relative
        try:
            text = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            missing.append(f"{relative}: missing file")
            continue
        for term in terms:
            if term not in text:
                missing.append(f"{relative}: missing {term!r}")
    return {
        "name": "candidate_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": [
            "RPO/RTO docs",
            "single-node SLO docs",
            "SDK/API compatibility docs",
            "upgrade/rollback docs",
        ],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("production candidate self-test failed: duplicate suite names")
        return 1
    required = {"production_hardening", "backup_rpo_rto_drill", "single_node_slo"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"production candidate self-test failed: missing suites {missing}")
        return 1
    if not DOC_REQUIREMENTS:
        print("production candidate self-test failed: no docs required")
        return 1
    print("production candidate self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/production-candidate")
    parser.add_argument("--report", default="target/production-candidate/report.json")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    repo = repo_root()
    root = repo / args.root
    report_path = repo / args.report
    root.mkdir(parents=True, exist_ok=True)

    started_at = utc_now()
    doc_suite = check_docs(repo)
    suites = [doc_suite, *[run_suite(repo, root, suite) for suite in SUITES]]
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
            "production_hardening_report": "target/production-hardening/report.json",
            "backup_drill_report": "target/backup-drill/report.json",
            "single_node_performance_report": "target/single-node-performance/report.json",
            "binary_release_archive": "target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz",
        },
        "boundary": {
            "proves": [
                "single-node production candidate gates are locally repeatable",
                "RPO/RTO and SLO boundaries are documented",
                "SDK/API compatibility and deprecation gates pass",
                "upgrade and rollback policy gates pass",
                "binary release package is buildable and valid",
            ],
            "does_not_prove": [
                "production v1.0 support claim",
                "managed service readiness",
                "online rolling upgrade support",
                "cross-platform binary matrix",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"production candidate check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"production candidate check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
