#!/usr/bin/env python3
"""Run the local single-node production hardening evidence matrix."""

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
        "make load-smoke-check",
        "make crash-fault-check",
        "cortexdb audit",
    ),
    "docs/CRASH_SIMULATION.md": (
        "make crash-fault-check",
        "target/crash-fault/report.json",
    ),
    "docs/UPGRADE_MIGRATION.md": (
        "make migration-compatibility-check",
        "compatibility",
    ),
    "docs/AUTH.md": (
        "rate_limited",
        "CORTEXDB_AUDIT_LOG_FILE",
        "cortexdb audit",
    ),
    "docs/ENCRYPTED_BACKUPS_DESIGN.md": (
        "envelope encryption",
        "Key Management",
        "Not Implemented",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "load_smoke",
        "command": ["make", "load-smoke-check"],
        "covers": ["concurrent local writes", "reads", "search", "ContextPack"],
    },
    {
        "name": "crash_fault",
        "command": ["make", "crash-fault-check"],
        "covers": ["partial WAL tail", "repair path", "corruption handling"],
    },
    {
        "name": "migration_compatibility",
        "command": ["make", "migration-compatibility-check"],
        "covers": ["storage/API/SDK compatibility fixture"],
    },
    {
        "name": "audit_hardening",
        "command": ["cargo", "test", "-p", "cortex-server", "audit"],
        "covers": ["audit classification", "audit JSONL redaction", "audit route metadata"],
    },
    {
        "name": "rate_limit_and_quota_boundary",
        "command": ["cargo", "test", "-p", "cortex-server", "rate_limit"],
        "covers": ["fixed-window rate limit", "typed 429 response"],
    },
    {
        "name": "cli_audit_tooling",
        "command": ["cargo", "test", "-p", "cortex-cli", "audit"],
        "covers": ["audit summary", "audit filters", "redaction check"],
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
        "name": "hardening_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": [
            "load/fault runbooks",
            "migration compatibility docs",
            "audit/rate-limit docs",
            "encrypted backups design",
        ],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("production hardening self-test failed: duplicate suite names")
        return 1
    required = {"load_smoke", "crash_fault", "migration_compatibility", "audit_hardening"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"production hardening self-test failed: missing suites {missing}")
        return 1
    print("production hardening self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/production-hardening")
    parser.add_argument("--report", default="target/production-hardening/report.json")
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
            "load_smoke_report": "target/load-smoke/report.json",
            "crash_fault_report": "target/crash-fault/report.json",
        },
        "boundary": {
            "proves": [
                "single-node load smoke runs locally",
                "crash/fault evidence is reproducible",
                "migration compatibility gate passes",
                "audit and rate-limit behavior is tested",
                "encrypted backups have a documented design boundary",
            ],
            "does_not_prove": [
                "production traffic SLO history",
                "implemented encrypted backups",
                "per-user quota enforcement",
                "tamper-evident audit chain",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"production hardening check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"production hardening check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
