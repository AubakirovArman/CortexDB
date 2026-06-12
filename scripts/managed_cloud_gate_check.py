#!/usr/bin/env python3
"""Validate local managed-cloud future-epic evidence gates."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


GATES: dict[str, dict[str, object]] = {
    "tenant-lifecycle": {
        "schema": "cortexdb.managed_cloud.tenant_lifecycle_gate.v1",
        "required_reports": {
            "tenant_recovery": ["checks.source_isolation", "checks.restored_isolation"],
            "observability": ["metrics_fields_checked", "ann_fields_checked"],
            "http_contract_ops": ["checks"],
        },
        "markers": [
            ("docs/MANAGED_CLOUD_DESIGN.md", "Tenant Lifecycle"),
            ("docs/SECURITY_MODEL.md", "make tenant-recovery-check"),
            ("Makefile", "cloud-tenant-lifecycle-check"),
        ],
    },
    "backup-restore": {
        "schema": "cortexdb.managed_cloud.backup_restore_gate.v1",
        "required_reports": {
            "backup_drill": ["restore_drill_trend", "evidence"],
            "backup_offsite": [
                "staged_copy_validated",
                "preflight_restore_completed",
                "payload_readable_after_stage",
            ],
            "tenant_recovery": ["checks.restored_isolation"],
        },
        "markers": [
            ("docs/MANAGED_CLOUD_DESIGN.md", "backup and restore"),
            ("docs/BACKUP_RESTORE.md", "make backup-drill-check"),
            ("docs/BACKUP_RESTORE.md", "make backup-offsite-check"),
            ("Makefile", "cloud-backup-restore-check"),
        ],
    },
    "upgrade": {
        "schema": "cortexdb.managed_cloud.upgrade_gate.v1",
        "required_reports": {
            "deployment_upgrade": ["release_workflow_checked", "docs_checked"],
        },
        "markers": [
            ("docs/MANAGED_CLOUD_DESIGN.md", "Upgrade and rollback"),
            ("docs/archive/UPGRADE_ROLLBACK.md", "make deployment-upgrade-check"),
            ("docs/archive/UPGRADE_MIGRATION.md", "make migration-policy-check"),
            ("Makefile", "cloud-upgrade-check"),
        ],
    },
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def load_report(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read evidence report {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"evidence report {path} is invalid JSON: {error}") from error


def report_passed(report: dict[str, Any]) -> bool:
    return report.get("status") in {"ok", "passed"}


def nested_value(report: dict[str, Any], path: list[str]) -> Any:
    value: Any = report
    for key in path:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def validate_report_shape(label: str, report: dict[str, Any], fields: list[str]) -> list[str]:
    failures: list[str] = []
    if not report_passed(report):
        failures.append(f"report {label} status is {report.get('status')!r}")
    for field in fields:
        if nested_value(report, field.split(".")) is None:
            failures.append(f"report {label} missing field {field!r}")
    return failures


def validate_markers(markers: list[tuple[str, str]]) -> list[str]:
    failures: list[str] = []
    for file_name, marker in markers:
        if marker not in read(Path(file_name)):
            failures.append(f"marker {marker!r} missing from {file_name}")
    return failures


def validate(gate: str, evidence: dict[str, Path]) -> dict[str, Any]:
    spec = GATES[gate]
    failures: list[str] = []
    failures.extend(validate_markers(spec["markers"]))  # type: ignore[arg-type]

    required_reports = spec["required_reports"]
    assert isinstance(required_reports, dict)
    loaded_reports: dict[str, dict[str, Any]] = {}
    for label, fields in required_reports.items():
        if label not in evidence:
            failures.append(f"missing --evidence {label}=<path>")
            continue
        report = load_report(evidence[label])
        loaded_reports[label] = report
        assert isinstance(fields, list)
        failures.extend(validate_report_shape(label, report, fields))

    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "managed_cloud_ready": False,
        "boundary": "local single-node managed-cloud prerequisites only; no hosted service claim",
        "evidence_reports": {label: str(path) for label, path in sorted(evidence.items())},
        "reports_checked": sorted(loaded_reports),
        "checks": {
            "markers": not any("marker " in failure for failure in failures),
            "reports_passed": all(report_passed(report) for report in loaded_reports.values()),
            "required_shapes_present": not any("missing field" in failure for failure in failures),
        },
        "failures": failures,
    }


def parse_evidence(values: list[str]) -> dict[str, Path]:
    evidence: dict[str, Path] = {}
    for value in values:
        if "=" not in value:
            raise RuntimeError(f"invalid evidence {value!r}; expected label=path")
        label, path = value.split("=", 1)
        if not label or not path:
            raise RuntimeError(f"invalid evidence {value!r}; expected label=path")
        evidence[label] = Path(path)
    return evidence


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATES))
    parser.add_argument("--evidence", action="append", default=[])
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    try:
        report = validate(args.gate, parse_evidence(args.evidence))
    except RuntimeError as error:
        print(f"managed cloud gate check failed: {error}", file=sys.stderr)
        return 1

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"managed cloud {args.gate} check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
