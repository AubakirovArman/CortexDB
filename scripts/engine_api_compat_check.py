#!/usr/bin/env python3
"""Run the external cortex-engine embedded API compatibility sample."""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path
from typing import Any


SAMPLE_MANIFEST = "examples/engine_api_compat/Cargo.toml"
SAMPLE_MAIN = "examples/engine_api_compat/src/main.rs"
DOCS = ("docs/ENGINE_API.md", "docs/ENGINE_API_COMPATIBILITY.md")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/engine-api-compat")
    parser.add_argument("--report", default="target/engine-api-compat/report.json")
    return parser.parse_args()


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
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def run_command(repo: Path, root: Path, name: str, command: list[str]) -> dict[str, Any]:
    started_ms = int(time.time() * 1000)
    result = subprocess.run(command, cwd=repo, capture_output=True, text=True, check=False)
    output = result.stdout
    if result.stderr:
        output += "\n--- stderr ---\n" + result.stderr
    log = root / f"{name}.log"
    log.write_text(output, encoding="utf-8")
    return {
        "name": name,
        "status": "passed" if result.returncode == 0 else "failed",
        "exit_code": result.returncode,
        "command": command,
        "log": str(log),
        "started_unix_ms": started_ms,
        "finished_unix_ms": int(time.time() * 1000),
    }


def check_static_files(repo: Path) -> dict[str, Any]:
    errors: list[str] = []
    for relative in (SAMPLE_MANIFEST, SAMPLE_MAIN, *DOCS):
        if not (repo / relative).exists():
            errors.append(f"{relative}: missing")
    manifest = (repo / SAMPLE_MANIFEST).read_text(encoding="utf-8")
    main = (repo / SAMPLE_MAIN).read_text(encoding="utf-8")
    for term in ("[workspace]", "cortex-engine", "cortex-core", "cortex-aql"):
        if term not in manifest:
            errors.append(f"{SAMPLE_MANIFEST}: missing {term}")
    for term in (
        "Database::open",
        "put_cell",
        "search_keyword",
        "context_pack_from_aql",
        "verify_fact_aql",
        "backup_path",
        "restore_from_backup",
    ):
        if term not in main:
            errors.append(f"{SAMPLE_MAIN}: missing {term}")
    return {
        "name": "static_contract",
        "status": "passed" if not errors else "failed",
        "errors": errors,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    root = repo / args.root
    root.mkdir(parents=True, exist_ok=True)
    static_suite = check_static_files(repo)
    suites = [
        static_suite,
        run_command(
            repo,
            root,
            "external_sample_run",
            ["cargo", "run", "--quiet", "--manifest-path", SAMPLE_MANIFEST],
        ),
    ]
    status = "passed" if all(suite["status"] == "passed" for suite in suites) else "failed"
    return {
        "schema_version": "cortexdb.engine_api_compat.report.v1",
        "status": status,
        "git_sha": git_sha(repo),
        "sample_manifest": SAMPLE_MANIFEST,
        "sample_main": SAMPLE_MAIN,
        "generated_unix_ms": int(time.time() * 1000),
        "suites": suites,
        "coverage": [
            "external crate compiles against path dependencies",
            "Database::open",
            "put/get",
            "keyword search",
            "ContextPack from AQL",
            "VERIFY FACT",
            "checkpoint",
            "backup and restore",
        ],
    }


def main() -> int:
    args = parse_args()
    report = build_report(args)
    output = repo_root() / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        print(f"engine API compatibility check failed: {output}")
        return 1
    print(f"engine API compatibility check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
