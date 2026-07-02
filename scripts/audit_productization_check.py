#!/usr/bin/env python3
"""Validate E07 audit log productization markers."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REPORT_SCHEMA = "cortexdb.audit_productization_report.v1"
MARKERS = {
    "server_schema_and_fields": [
        ("crates/cortex-server/src/audit.rs", 'AUDIT_SCHEMA_VERSION_V2: &str = "cortexdb.audit.v2"'),
        ("crates/cortex-server/src/audit.rs", "mac_key_id"),
        ("crates/cortex-server/src/audit.rs", "event_mac"),
        ("crates/cortex-server/src/audit/sink.rs", "record.schema_version = AUDIT_SCHEMA_VERSION_V2"),
        ("crates/cortex-server/src/audit.rs", "scope_decision"),
        ("crates/cortex-server/src/audit.rs", "AuditAction::Verify"),
        ("crates/cortex-server/src/audit/llm.rs", "llm_inference_decision"),
    ],
    "rotation_and_fsync": [
        ("crates/cortex-server/src/audit/sink.rs", "should_rotate"),
        ("crates/cortex-server/src/audit/sink.rs", "rotated_path"),
        ("crates/cortex-server/src/config.rs", "AuditLogFsyncPolicy"),
        ("crates/cortex-server/src/main.rs", "CORTEXDB_AUDIT_LOG_ROTATE_BYTES"),
        ("crates/cortex-server/src/main.rs", "CORTEXDB_AUDIT_LOG_FSYNC"),
        ("crates/cortex-server/src/audit_tests.rs", "audit_sink_rotates_active_jsonl"),
    ],
    "denied_access_evidence": [
        (
            "crates/cortex-server/src/tests/security_redaction_tests.rs",
            "denied_ingestion_audit_event_does_not_leak_query_body_or_token",
        ),
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "\"denied\""),
    ],
    "cli_and_siem": [
        ("crates/cortex-cli/src/cli_audit.rs", "scope_decision"),
        ("crates/cortex-cli/src/cli_audit_siem.rs", "scope_decision"),
        ("crates/cortex-cli/src/cli_audit_chain.rs", "scope_decision"),
        ("docs/archive/AUDIT_EXPORT_RETENTION_POLICY.md", "cortexdb.siem.audit.v1"),
    ],
    "format_doc": [
        ("docs/AUDIT_LOG_FORMAT.md", "cortexdb.audit.v2"),
        ("docs/AUDIT_LOG_FORMAT.md", "CORTEXDB_AUDIT_MAC_KEY_HEX"),
        ("docs/AUDIT_LOG_FORMAT.md", "--mac-key-file"),
        ("docs/AUDIT_LOG_FORMAT.md", "scope_decision"),
        ("docs/AUDIT_LOG_FORMAT.md", "CORTEXDB_AUDIT_LOG_ROTATE_BYTES"),
        ("docs/AUDIT_LOG_FORMAT.md", "CORTEXDB_AUDIT_LOG_FSYNC"),
        ("docs/AUTH.md", "AUDIT_LOG_FORMAT.md"),
    ],
}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def validate(repo: Path) -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for check, markers in MARKERS.items():
        passed = True
        for file_name, marker in markers:
            try:
                text = (repo / file_name).read_text(encoding="utf-8")
            except OSError as error:
                failures.append(f"{check}: failed to read {file_name}: {error}")
                passed = False
                continue
            if marker not in text:
                failures.append(f"{check}: marker {marker!r} missing from {file_name}")
                passed = False
        checks[check] = passed
    return {
        "schema_version": REPORT_SCHEMA,
        "status": "passed" if not failures else "failed",
        "checks": checks,
        "checked_markers": sum(len(markers) for markers in MARKERS.values()),
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/audit-productization/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = repo_root()
    report = validate(repo)
    output = repo / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"audit productization report: {report['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
