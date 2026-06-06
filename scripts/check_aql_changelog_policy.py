#!/usr/bin/env python3
"""Validate the AQL grammar changelog registry and examples."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


SCHEMA = "cortexdb.aql_grammar_change_registry.v1"
REPORT_SCHEMA = "cortexdb.aql_changelog_policy_report.v1"


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def string_field(data: dict[str, Any], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{key}: expected non-empty string")
    return value


def string_list(data: dict[str, Any], key: str) -> list[str]:
    value = data.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ValueError(f"{key}: expected non-empty string list")
    return list(value)


def validate_entry(repo: Path, changelog: str, grammar_doc: str, entry: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    change_id = string_field(entry, "change_id")
    example = string_field(entry, "example")
    string_field(entry, "introduced_in")
    string_field(entry, "classification")
    string_field(entry, "summary")
    grammar_terms = string_list(entry, "grammar_terms")
    tests = string_list(entry, "tests")

    if change_id not in changelog:
        failures.append(f"{change_id}: missing changelog anchor")
    if example not in changelog:
        failures.append(f"{change_id}: missing runnable example in changelog")
    for term in grammar_terms:
        if term not in grammar_doc and term not in changelog:
            failures.append(f"{change_id}: missing grammar term {term!r}")
    for relative in tests:
        if not (repo / relative).is_file():
            failures.append(f"{change_id}: missing test reference {relative}")
    return failures


def validate_registry(repo: Path, registry: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if registry.get("schema_version") != SCHEMA:
        failures.append(f"schema_version must be {SCHEMA}")

    policy = registry.get("policy")
    if not isinstance(policy, dict):
        failures.append("policy must be an object")
        return failures

    changelog_path = repo / string_field(policy, "changelog")
    grammar_path = repo / string_field(policy, "grammar_doc")
    gate = string_field(policy, "compatibility_gate")
    rule = string_field(policy, "rule")
    if not gate.startswith("make "):
        failures.append("policy.compatibility_gate must be a make target")
    for word in ("changelog", "example", "test"):
        if word not in rule.lower():
            failures.append(f"policy.rule must mention {word}")

    try:
        changelog = changelog_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        failures.append(f"missing changelog file {changelog_path}")
        changelog = ""
    try:
        grammar_doc = grammar_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        failures.append(f"missing grammar doc {grammar_path}")
        grammar_doc = ""

    entries = registry.get("entries")
    if not isinstance(entries, list) or not entries:
        failures.append("entries must be a non-empty list")
        return failures

    seen: set[str] = set()
    for index, value in enumerate(entries):
        if not isinstance(value, dict):
            failures.append(f"entries[{index}] must be an object")
            continue
        try:
            change_id = string_field(value, "change_id")
            if change_id in seen:
                failures.append(f"duplicate change_id {change_id}")
            seen.add(change_id)
            failures.extend(validate_entry(repo, changelog, grammar_doc, value))
        except Exception as error:  # noqa: BLE001 - policy gate aggregates failures.
            failures.append(f"entries[{index}]: {error}")
    return failures


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", default="fixtures/aql/grammar_change_registry_v1.json")
    parser.add_argument("--report", default="target/aql-changelog-policy/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = repo_root()
    registry_path = repo / args.registry
    try:
        registry = read_json(registry_path)
        failures = validate_registry(repo, registry)
    except Exception as error:  # noqa: BLE001 - policy gate reports all failures.
        failures = [str(error)]

    report = {
        "schema_version": REPORT_SCHEMA,
        "status": "passed" if not failures else "failed",
        "registry": args.registry,
        "changelog": registry.get("policy", {}).get("changelog") if "registry" in locals() else None,
        "grammar_doc": registry.get("policy", {}).get("grammar_doc") if "registry" in locals() else None,
        "entries_checked": len(registry.get("entries", [])) if "registry" in locals() else 0,
        "failures": failures,
    }
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"AQL changelog policy report: {report_path}")
    for failure in failures:
        print(f"failure: {failure}", file=sys.stderr)
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
