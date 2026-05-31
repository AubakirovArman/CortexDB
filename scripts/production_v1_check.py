#!/usr/bin/env python3
"""Run the local single-node Production v1.0 evidence matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "docs/PRODUCTION_V1.md": (
        "single-node production",
        "Stable API/SDK",
        "Supported Backup/Restore",
        "Distributed Production Is Out Of Scope",
    ),
    "docs/API_COMPATIBILITY.md": (
        "OpenAPI",
        "SDK",
        "compatibility",
    ),
    "docs/SDK_RELEASE.md": (
        "sdk-release-contract-check",
        "deprecation",
        "version",
    ),
    "docs/BACKUP_RESTORE.md": (
        "make backup-drill-check",
        "make backup-offsite-check",
        "Release Evidence",
    ),
    "docs/OPERATIONS.md": (
        "make production-v1-check",
        "make production-candidate-check",
        "cortexdb validate",
    ),
    "docs/REPLICATION.md": (
        "experimental",
        "not production",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "production_candidate",
        "command": ["make", "production-candidate-check"],
        "covers": ["RPO/RTO", "SLO", "compatibility", "binary release"],
    },
    {
        "name": "release_check",
        "command": ["make", "release-check"],
        "covers": ["full alpha/release matrix", "binary package", "backup/fault evidence"],
    },
    {
        "name": "openapi_contract",
        "command": ["make", "openapi-contract-check"],
        "covers": ["stable HTTP API contract"],
    },
    {
        "name": "sdk_release_contract",
        "command": ["make", "sdk-release-contract-check"],
        "covers": ["stable SDK release metadata", "version lock-step"],
    },
    {
        "name": "sdk_deprecation_policy",
        "command": ["make", "sdk-deprecation-check"],
        "covers": ["SDK/API lifecycle policy", "deprecated route policy"],
    },
    {
        "name": "backup_restore_support",
        "command": ["make", "backup-drill-check"],
        "covers": ["supported local backup/restore drill"],
    },
    {
        "name": "backup_offsite_support",
        "command": ["make", "backup-offsite-check"],
        "covers": ["validated local offsite staging"],
    },
    {
        "name": "public_claims",
        "command": ["make", "public-claims-check"],
        "covers": ["single-node wording", "production overclaim guard"],
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
        "name": "production_v1_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": [
            "single-node production claim",
            "stable API/SDK docs",
            "supported backup/restore docs",
            "complete operational docs",
            "distributed production out-of-scope docs",
        ],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("production v1 self-test failed: duplicate suite names")
        return 1
    required = {"production_candidate", "release_check", "public_claims"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"production v1 self-test failed: missing suites {missing}")
        return 1
    if not DOC_REQUIREMENTS:
        print("production v1 self-test failed: no docs required")
        return 1
    print("production v1 self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/production-v1")
    parser.add_argument("--report", default="target/production-v1/report.json")
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
            "production_candidate_report": "target/production-candidate/report.json",
            "release_evidence": "docs/RELEASE_EVIDENCE.md",
            "release_archive": "target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz",
        },
        "boundary": {
            "proves": [
                "single-node production-v1 evidence gates pass locally",
                "stable API/SDK compatibility gates pass",
                "supported backup/restore gates pass",
                "operational docs are complete for the single-node boundary",
                "distributed production is explicitly out of scope",
            ],
            "does_not_prove": [
                "managed cloud service readiness",
                "production distributed consensus",
                "cross-platform release artifacts beyond this host",
                "external security certification",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"production v1 check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"production v1 check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
