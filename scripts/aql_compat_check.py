#!/usr/bin/env python3
"""Run the AQL v0.4 compatibility evidence matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "docs/AQL_V0_4.md": (
        "EXPLAIN RETRIEVE CONTEXT",
        "LIMIT",
        "REQUIRE",
        "Bind errors expose stable codes",
    ),
    "docs/AQL_COMPATIBILITY.md": (
        "malformed AQL",
        "forbidden scope",
        "unknown field",
        "LIMIT and REQUIRE",
        "explain snapshots",
    ),
    "docs/AQL_CHANGELOG.md": (
        "AQL v0.4",
        "breaking change",
        "grammar_change_registry_v1.json",
        "aql-v0.4-retrieve-context",
        "aql-v0.4-require-thresholds",
        "golden tests",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "aql_v0_4_golden",
        "command": ["cargo", "test", "-p", "cortex-aql", "--test", "aql_v0_4_golden_tests"],
        "covers": [
            "parse shape",
            "bound retrieve bytecode",
            "EXPLAIN RETRIEVE CONTEXT",
            "stable parse/bind errors",
        ],
    },
    {
        "name": "parser_contract",
        "command": ["cargo", "test", "-p", "cortex-aql", "--test", "parser_tests"],
        "covers": ["malformed AQL", "LIMIT and REQUIRE", "WHERE precedence", "integer errors"],
    },
    {
        "name": "binder_contract",
        "command": ["cargo", "test", "-p", "cortex-aql", "--test", "binder_hardening_tests"],
        "covers": ["forbidden scope", "unknown scope safe diagnostics", "IN filters"],
    },
    {
        "name": "aql_stabilization",
        "command": ["cargo", "test", "-p", "cortex-aql", "--test", "aql_stabilization_tests"],
        "covers": ["default mode/budget", "LIMIT clamp", "REQUIRE thresholds"],
    },
    {
        "name": "aql_changelog_policy",
        "command": [
            "python3",
            "scripts/check_aql_changelog_policy.py",
            "--report",
            "target/aql-compat/aql_changelog_policy_report.json",
        ],
        "covers": ["AQL grammar change changelog entries", "SQL examples for grammar changes"],
    },
    {
        "name": "http_invalid_aql_error",
        "command": ["cargo", "test", "-p", "cortex-server", "error_taxonomy_invalid_aql_has_stable_code"],
        "covers": ["HTTP invalid_aql code for SDK callers"],
    },
    {
        "name": "http_unknown_field_error",
        "command": ["cargo", "test", "-p", "cortex-server", "error_taxonomy_unknown_field_has_stable_code"],
        "covers": ["HTTP unknown_field code for SDK callers"],
    },
    {
        "name": "http_unsupported_operator_error",
        "command": ["cargo", "test", "-p", "cortex-server", "error_taxonomy_unsupported_operator_has_stable_code"],
        "covers": ["HTTP unsupported_operator code for SDK callers"],
    },
    {
        "name": "http_permission_denied_error",
        "command": ["cargo", "test", "-p", "cortex-server", "error_taxonomy_denied_scope_has_stable_code"],
        "covers": ["HTTP permission_denied code for SDK callers"],
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
        "name": "aql_compat_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": ["AQL v0.4 grammar docs", "AQL changelog policy", "client error classes"],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("AQL compatibility self-test failed: duplicate suite names")
        return 1
    required = {"aql_v0_4_golden", "parser_contract", "binder_contract", "aql_changelog_policy"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"AQL compatibility self-test failed: missing suites {missing}")
        return 1
    if not DOC_REQUIREMENTS:
        print("AQL compatibility self-test failed: no doc requirements")
        return 1
    print("AQL compatibility self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/aql-compat")
    parser.add_argument("--report", default="target/aql-compat/report.json")
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
            "grammar_doc": "docs/AQL_V0_4.md",
            "compatibility_doc": "docs/AQL_COMPATIBILITY.md",
            "changelog": "docs/AQL_CHANGELOG.md",
            "golden_tests": "crates/cortex-aql/tests/aql_v0_4_golden_tests.rs",
        },
        "boundary": {
            "proves": [
                "AQL v0.4 parser and binder compatibility tests pass",
                "EXPLAIN RETRIEVE CONTEXT parses and binds like its inner retrieve",
                "malformed AQL, permission denied, unknown field, unsupported operator, LIMIT, and REQUIRE are covered",
                "HTTP error codes remain distinguishable for SDK callers",
            ],
            "does_not_prove": [
                "future AQL v0.5 compatibility",
                "semantic ranking quality",
                "new comparators beyond the documented v0.4 binder surface",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"AQL compatibility check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"AQL compatibility check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
