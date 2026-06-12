#!/usr/bin/env python3
"""Validate CortexDB unified versioning policy."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


SCHEMA = "cortexdb.versioning_policy.v1"
REQUIRED_SURFACES = {"http_api", "sdk", "storage_format", "aql"}
REQUIRED_PROCESS_WORDS = {"changelog", "migration", "tests", "gate", "release notes"}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def string_field(data: dict[str, Any], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value:
        raise ValueError(f"{key}: expected non-empty string")
    return value


def string_list(data: dict[str, Any], key: str) -> list[str]:
    value = data.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ValueError(f"{key}: expected non-empty string list")
    return list(value)


def validate_surface(repo: Path, surface: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    name = string_field(surface, "name")
    for key in ("version_source", "current_contract", "changelog", "compatibility_gate"):
        string_field(surface, key)
    examples = string_list(surface, "breaking_change_examples")
    if len(examples) < 3:
        failures.append(f"{name}: expected at least 3 breaking change examples")
    for key in ("version_source", "changelog"):
        path = repo / surface[key]
        if not path.is_file():
            failures.append(f"{name}: missing {key} file {surface[key]}")
    gate = surface["compatibility_gate"]
    if not gate.startswith("make "):
        failures.append(f"{name}: compatibility_gate must be a make target")
    return failures


def validate_policy(repo: Path, policy: dict[str, Any], markdown: Path) -> list[str]:
    failures: list[str] = []
    if policy.get("schema_version") != SCHEMA:
        failures.append(f"schema_version must be {SCHEMA}")
    surfaces = policy.get("surfaces")
    if not isinstance(surfaces, list):
        failures.append("surfaces must be a list")
        surfaces = []

    names: set[str] = set()
    for surface in surfaces:
        if not isinstance(surface, dict):
            failures.append("surface entry must be an object")
            continue
        try:
            name = string_field(surface, "name")
            if name in names:
                failures.append(f"duplicate surface {name}")
            names.add(name)
            failures.extend(validate_surface(repo, surface))
        except Exception as error:  # noqa: BLE001 - policy gate aggregates failures.
            failures.append(str(error))

    missing = sorted(REQUIRED_SURFACES - names)
    if missing:
        failures.append(f"missing required surfaces: {missing}")

    process = " ".join(string_list(policy, "breaking_change_process")).lower()
    for word in REQUIRED_PROCESS_WORDS:
        if word not in process:
            failures.append(f"breaking_change_process must mention {word}")

    markdown_text = markdown.read_text(encoding="utf-8")
    for name in REQUIRED_SURFACES:
        if name.replace("_", " ") not in markdown_text.lower() and name not in markdown_text:
            failures.append(f"markdown policy does not mention {name}")
    return failures


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", default="docs/VERSIONING_POLICY.json")
    parser.add_argument("--markdown", default="docs/archive/VERSIONING_POLICY.md")
    parser.add_argument("--report", default="target/versioning-policy/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = repo_root()
    try:
        policy = read_json(repo / args.policy)
        failures = validate_policy(repo, policy, repo / args.markdown)
    except Exception as error:  # noqa: BLE001 - release gate reports failures.
        failures = [str(error)]
    report = {
        "schema_version": "cortexdb.versioning_policy_report.v1",
        "status": "passed" if not failures else "failed",
        "policy": args.policy,
        "markdown": args.markdown,
        "failures": failures,
    }
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"versioning policy report: {report_path}")
    for failure in failures:
        print(f"failure: {failure}", file=sys.stderr)
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
