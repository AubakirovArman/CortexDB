#!/usr/bin/env python3
"""Run the local Beta Release Candidate evidence matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "docs/OPERATIONS.md": (
        "Core Alpha",
        "backup",
        "restore",
        "validate",
        "metrics",
    ),
    "docs/SECURITY_MODEL.md": (
        "Core Alpha",
        "Token roles",
        "AgentView",
        "Not Yet Production Security",
    ),
    "docs/INGESTION.md": (
        "ingest",
        "job",
        "retry",
        "cancel",
    ),
    "docs/DASHBOARD_UI.md": (
        "developer console",
        "dashboard-release-check",
        "operational",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "beta_foundation",
        "command": ["make", "beta-foundation-check"],
        "covers": ["SDK/API/ContextPack/VERIFY/Search foundation evidence"],
    },
    {
        "name": "backup_restore_drill",
        "command": ["make", "backup-drill-check"],
        "covers": ["backup creation", "restore readback", "validated recovered database"],
    },
    {
        "name": "backup_offsite_stage",
        "command": ["make", "backup-offsite-check"],
        "covers": ["validated local offsite staging", "backup publication preflight"],
    },
    {
        "name": "security_model_tests",
        "command": ["cargo", "test", "-p", "cortex-server", "security_tests"],
        "covers": ["auth", "size limits", "tenant validation", "audit redaction"],
    },
    {
        "name": "auth_policy_tests",
        "command": ["cargo", "test", "-p", "cortex-server", "auth_policy_tests"],
        "covers": ["admin/data token split", "agent-scoped routes", "policy reload"],
    },
    {
        "name": "ingestion_jobs",
        "command": ["cargo", "test", "-p", "cortex-server", "ingest"],
        "covers": ["ingestion endpoints", "job lifecycle", "empty input behavior"],
    },
    {
        "name": "dashboard_operational_view",
        "command": ["make", "dashboard-release-check"],
        "covers": ["dashboard build", "standalone package", "release artifact validation"],
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
        "name": "operational_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": [
            "operations docs",
            "security model v1 docs",
            "ingestion job docs",
            "dashboard operations docs",
        ],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("beta rc self-test failed: duplicate suite names")
        return 1
    required = {"beta_foundation", "backup_restore_drill", "security_model_tests"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"beta rc self-test failed: missing suites {missing}")
        return 1
    if not DOC_REQUIREMENTS:
        print("beta rc self-test failed: no docs required")
        return 1
    print("beta rc self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/beta-rc")
    parser.add_argument("--report", default="target/beta-rc/report.json")
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
            "backup_drill_report": "target/backup-drill/report.json",
            "backup_offsite_report": "target/backup-offsite/report.json",
            "dashboard_archive": "target/dashboard/dashboard-v1.tar.gz",
        },
        "boundary": {
            "proves": [
                "backup and restore evidence is locally repeatable",
                "operational, security, ingestion, and dashboard docs are present",
                "security and auth policy tests pass",
                "ingestion job lifecycle tests pass",
                "dashboard release package is buildable and valid",
            ],
            "does_not_prove": [
                "production security certification",
                "managed service operational readiness",
                "full product web UI maturity",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"beta rc check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"beta rc check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
