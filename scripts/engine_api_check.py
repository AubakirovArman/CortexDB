#!/usr/bin/env python3
"""Run the cortex-engine public API stability evidence matrix."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


FREEZE_FIXTURE = "fixtures/engine/public_api_freeze_v1.json"

DOC_REQUIREMENTS: dict[str, tuple[str, ...]] = {
    "docs/ENGINE_API.md": (
        "Stable Embedded API",
        "Internal APIs",
        "Compatibility Gate",
        "cortex_engine::Database",
    ),
    "docs/MODULE_OWNERSHIP.md": (
        "Stable facade",
        "Internal modules",
        "cortex-engine",
    ),
    "docs/CORE_ENGINE.md": (
        "cortex-engine",
        "Database::open",
        "PutCell",
    ),
}

SUITES: tuple[dict[str, Any], ...] = (
    {
        "name": "public_api_compile",
        "command": ["cargo", "test", "-p", "cortex-engine", "--test", "public_api"],
        "covers": ["stable public re-exports", "Database facade compile path"],
    },
    {
        "name": "engine_doc_tests",
        "command": ["cargo", "test", "-p", "cortex-engine", "--doc"],
        "covers": ["public documentation examples compile"],
    },
    {
        "name": "engine_docs_build",
        "command": ["cargo", "doc", "-p", "cortex-engine", "--no-deps"],
        "covers": ["rustdoc builds for the engine crate"],
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
        "name": "engine_api_docs",
        "status": "passed" if not missing else "failed",
        "missing": missing,
        "covers": ["stable vs internal API docs", "module ownership docs"],
    }


def read_json(repo: Path, relative: str) -> dict[str, Any]:
    value = json.loads((repo / relative).read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{relative} must contain a JSON object")
    return value


def check_public_api_freeze(repo: Path) -> dict[str, Any]:
    errors: list[str] = []
    try:
        fixture = read_json(repo, FREEZE_FIXTURE)
    except (FileNotFoundError, ValueError, json.JSONDecodeError) as error:
        return {
            "name": "engine_public_api_freeze",
            "status": "failed",
            "errors": [f"{FREEZE_FIXTURE}: {error}"],
            "covers": ["machine-readable engine API freeze contract"],
        }

    if fixture.get("schema_version") != "cortexdb.engine_public_api_freeze.v1":
        errors.append("fixture schema_version must be cortexdb.engine_public_api_freeze.v1")
    if fixture.get("freeze_id") != "engine-public-api-freeze-v1":
        errors.append("fixture freeze_id must be engine-public-api-freeze-v1")

    lib_rs = (repo / "crates/cortex-engine/src/lib.rs").read_text(encoding="utf-8")
    public_test = (repo / "crates/cortex-engine/tests/public_api.rs").read_text(encoding="utf-8")
    engine_api_doc = (repo / "docs/ENGINE_API.md").read_text(encoding="utf-8")
    makefile = (repo / "Makefile").read_text(encoding="utf-8")

    for symbol in fixture.get("stable_facade_symbols", []):
        if not isinstance(symbol, str):
            errors.append("stable_facade_symbols entries must be strings")
            continue
        if symbol not in lib_rs:
            errors.append(f"lib.rs: stable facade symbol {symbol} is not exported")
        if symbol not in public_test:
            errors.append(f"public_api.rs: stable facade symbol {symbol} is not covered")
        if symbol not in engine_api_doc:
            errors.append(f"ENGINE_API.md: stable facade symbol {symbol} is not documented")

    for module in fixture.get("private_modules", []):
        if not isinstance(module, str):
            errors.append("private_modules entries must be strings")
            continue
        pattern = rf"^pub\s+mod\s+{re.escape(module)};"
        if re.search(pattern, lib_rs, re.M):
            errors.append(f"lib.rs: internal module {module} must not be public")

    for relative in fixture.get("required_docs", []):
        if not isinstance(relative, str):
            errors.append("required_docs entries must be strings")
        elif not (repo / relative).exists():
            errors.append(f"{relative}: missing required API document")

    for relative, terms in fixture.get("rustdoc_examples", {}).items():
        path = repo / relative
        if not path.exists():
            errors.append(f"{relative}: missing rustdoc source")
            continue
        text = path.read_text(encoding="utf-8")
        if "```" not in text:
            errors.append(f"{relative}: missing rustdoc code fence")
        for term in terms:
            if term not in text:
                errors.append(f"{relative}: missing rustdoc example term {term!r}")

    for gate in fixture.get("required_evidence_gates", []):
        if not isinstance(gate, str):
            errors.append("required_evidence_gates entries must be strings")
            continue
        target = gate.removeprefix("make ")
        if f"{target}:" not in makefile:
            errors.append(f"Makefile: missing evidence gate target {target}")

    return {
        "name": "engine_public_api_freeze",
        "status": "passed" if not errors else "failed",
        "errors": errors,
        "fixture": FREEZE_FIXTURE,
        "stable_facade_symbols": fixture.get("stable_facade_symbols", []),
        "private_modules": fixture.get("private_modules", []),
        "covers": [
            "machine-readable engine API freeze contract",
            "stable facade symbols are exported, documented, and compile-tested",
            "known internal helper modules remain private",
            "rustdoc examples exist for the embedded database facade",
        ],
    }


def write_report(report_path: Path, report: dict[str, Any]) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("engine API self-test failed: duplicate suite names")
        return 1
    required = {"public_api_compile", "engine_doc_tests", "engine_docs_build"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"engine API self-test failed: missing suites {missing}")
        return 1
    if not DOC_REQUIREMENTS:
        print("engine API self-test failed: no doc requirements")
        return 1
    repo = repo_root()
    if not (repo / FREEZE_FIXTURE).exists():
        print(f"engine API self-test failed: missing {FREEZE_FIXTURE}")
        return 1
    print("engine API self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/engine-api")
    parser.add_argument("--report", default="target/engine-api/report.json")
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
    freeze_suite = check_public_api_freeze(repo)
    suites = [doc_suite, freeze_suite, *[run_suite(repo, root, suite) for suite in SUITES]]
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
            "public_api_test": "crates/cortex-engine/tests/public_api.rs",
            "engine_api_doc": "docs/ENGINE_API.md",
            "module_ownership_doc": "docs/MODULE_OWNERSHIP.md",
            "public_api_freeze_fixture": FREEZE_FIXTURE,
        },
        "boundary": {
            "proves": [
                "stable embedded engine API docs exist",
                "public API freeze fixture matches crate-root exports and compile coverage",
                "public API compile test passes",
                "engine doctests compile",
                "engine rustdoc builds",
            ],
            "does_not_prove": [
                "all internal modules are stable",
                "no future breaking changes",
                "C ABI or non-Rust embedded API stability",
            ],
        },
    }
    write_report(report_path, report)

    if status != "passed":
        print(f"engine API check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"engine API check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
