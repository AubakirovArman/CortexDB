#!/usr/bin/env python3
"""Validate CLI productization evidence."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


COMMANDS = [
    "doctor",
    "completions",
    "stats",
    "validate",
    "context",
    "verify",
    "search",
    "search-vector-eval",
    "audit",
]


def cargo_run(*args: str) -> str:
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "cortex-cli", "--", *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return result.stdout


def validate() -> dict[str, object]:
    failures: list[str] = []
    help_output = cargo_run("--help")
    version_output = cargo_run("version")
    bash_completion = cargo_run("completions", "bash")
    docs = Path("docs/CLI.md").read_text(encoding="utf-8")

    for command in COMMANDS:
        if command not in help_output:
            failures.append(f"help output is missing command: {command}")
        if command not in docs:
            failures.append(f"docs/CLI.md is missing command: {command}")
    if not version_output.startswith("cortexdb "):
        failures.append("version output must start with 'cortexdb '")
    if "_cortexdb" not in bash_completion or "doctor" not in bash_completion:
        failures.append("bash completion output is missing expected command data")
    if "cortexdb doctor ./db" not in docs:
        failures.append("docs/CLI.md must document doctor quick usage")
    if "cortexdb completions bash" not in docs:
        failures.append("docs/CLI.md must document completions quick usage")

    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "commands_checked": COMMANDS,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except (OSError, RuntimeError) as error:
        print(f"cli product check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"cli product check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
