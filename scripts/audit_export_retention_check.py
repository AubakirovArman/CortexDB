#!/usr/bin/env python3
"""Validate CortexDB local audit export, retention, and redaction policy."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


POLICY_SCHEMA = "cortexdb.audit_export_retention_policy.v1"
REPORT_SCHEMA = "cortexdb.audit_export_retention_report.v1"
REQUIRED_EXPORT = "local_siem_jsonl"
REQUIRED_RETENTION_CLASSES = {
    "local_audit_jsonl",
    "siem_export_jsonl",
    "local_only_raw_security_debug",
}
REQUIRED_FORBIDDEN_FIELDS = {
    "authorization",
    "auth_header",
    "bearer_token",
    "body",
    "payload",
    "query",
    "query_string",
    "prompt",
    "provider_response",
    "api_key",
    "secret",
}
REQUIRED_SAFE_FIELDS = {
    "principal_id",
    "auth_role",
    "auth_agent_id",
    "request_id",
    "audit_action",
    "chain_id",
    "sequence",
    "prev_hash",
    "event_hash",
    "llm.outcome",
    "llm.reason",
    "llm.model",
}
MARKERS = [
    ("docs/AUTH.md", "audit-export-siem"),
    ("docs/archive/AUDIT_EXPORT_RETENTION_POLICY.md", "cortexdb.siem.audit.v1"),
    ("docs/archive/AUDIT_EXPORT_RETENTION_POLICY.md", "local_only_raw_security_debug"),
    ("docs/AUDIT_EXPORT_RETENTION_POLICY.json", POLICY_SCHEMA),
    ("crates/cortex-cli/src/cli_audit_siem.rs", "cortexdb.siem.audit.v1"),
    ("crates/cortex-cli/src/cli_audit_siem_tests.rs", "audit_export_siem_writes_normalized_jsonl"),
    ("crates/cortex-cli/src/cli_audit_siem_tests.rs", "audit_export_siem_rejects_redaction_violations"),
    ("crates/cortex-cli/src/cli_audit_siem_tests.rs", "audit_export_siem_rejects_chain_violations"),
]


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def string_set(data: dict[str, Any], key: str) -> set[str]:
    value = data.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ValueError(f"{key}: expected non-empty string list")
    return set(value)


def validate_policy(policy: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if policy.get("schema_version") != POLICY_SCHEMA:
        failures.append(f"schema_version must be {POLICY_SCHEMA}")

    export_channels = policy.get("export_channels")
    if not isinstance(export_channels, list) or not export_channels:
        failures.append("export_channels must be a non-empty list")
    else:
        exports = {
            entry.get("name"): entry for entry in export_channels if isinstance(entry, dict)
        }
        export = exports.get(REQUIRED_EXPORT)
        if not isinstance(export, dict):
            failures.append(f"missing export channel {REQUIRED_EXPORT}")
        else:
            if "audit-export-siem" not in str(export.get("command", "")):
                failures.append("local_siem_jsonl command must use audit-export-siem")
            if export.get("output_schema") != "cortexdb.siem.audit.v1":
                failures.append("local_siem_jsonl output schema mismatch")
            if export.get("requires_redaction_check") is not True:
                failures.append("local_siem_jsonl must require redaction check")
            if export.get("supports_chain_check") is not True:
                failures.append("local_siem_jsonl must support chain check")

    classes = policy.get("retention_classes")
    if not isinstance(classes, list) or not classes:
        failures.append("retention_classes must be a non-empty list")
    else:
        names = {entry.get("name") for entry in classes if isinstance(entry, dict)}
        missing = sorted(REQUIRED_RETENTION_CLASSES - names)
        if missing:
            failures.append(f"missing retention classes: {missing}")
        for entry in classes:
            if not isinstance(entry, dict):
                failures.append("retention class must be an object")
                continue
            name = entry.get("name")
            if not isinstance(name, str) or not name:
                failures.append("retention class name must be non-empty")
            for key in ["minimum_retention", "storage_boundary", "rotation_policy"]:
                if not isinstance(entry.get(key), str) or not entry.get(key):
                    failures.append(f"{name}: {key} must be non-empty")
            for key in ["contains_payloads", "contains_tokens"]:
                if not isinstance(entry.get(key), bool):
                    failures.append(f"{name}: {key} must be boolean")

    redaction = policy.get("redaction_policy")
    if not isinstance(redaction, dict):
        failures.append("redaction_policy must be an object")
    else:
        missing_forbidden = sorted(REQUIRED_FORBIDDEN_FIELDS - string_set(redaction, "forbidden_fields"))
        if missing_forbidden:
            failures.append(f"missing forbidden redaction fields: {missing_forbidden}")
        missing_safe = sorted(REQUIRED_SAFE_FIELDS - string_set(redaction, "safe_fields"))
        if missing_safe:
            failures.append(f"missing safe audit fields: {missing_safe}")
    return failures


def validate_markers(repo: Path) -> list[str]:
    failures: list[str] = []
    for file_name, marker in MARKERS:
        text = (repo / file_name).read_text(encoding="utf-8")
        if marker not in text:
            failures.append(f"{file_name}: missing marker {marker!r}")
    return failures


def run(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    policy = read_json(repo / args.policy)
    failures = validate_policy(policy)
    failures.extend(validate_markers(repo))
    report = {
        "schema_version": REPORT_SCHEMA,
        "status": "passed" if not failures else "failed",
        "policy": args.policy,
        "export_channel_count": len(policy.get("export_channels", [])),
        "retention_class_count": len(policy.get("retention_classes", [])),
        "checked_markers": len(MARKERS),
        "failures": failures,
    }
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", default="docs/AUDIT_EXPORT_RETENTION_POLICY.json")
    parser.add_argument("--report", default="target/audit-export-retention/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    try:
        report = run(parse_args(argv))
    except Exception as error:  # noqa: BLE001 - release gate must report failures.
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"audit export retention report: {report['status']}")
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
