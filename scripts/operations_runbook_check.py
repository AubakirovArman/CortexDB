#!/usr/bin/env python3
"""Validate that the operations runbook is self-contained for operators."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS: dict[str, tuple[str, ...]] = {
    "install_and_start": (
        "Install from release archive",
        "cortex-server ./data 127.0.0.1:8181",
        "CORTEXDB_AUTH_TOKEN",
        "cortexdb doctor ./data",
    ),
    "health_auth_and_data": (
        "/v1/health",
        "Authorization: Bearer",
        "cortexdb put ./data",
        "cortexdb get ./data",
        "cortexdb validate ./data",
        "cortexdb stats ./data",
    ),
    "backup_restore": (
        "cortexdb backup ./data",
        "cortexdb restore ./backups",
        "cortexdb backup-encrypted ./data",
        "cortexdb restore-encrypted",
        "cortexdb backup-offsite-stage",
        "make backup-restore-production-pack-check",
    ),
    "repair_and_corruption": (
        "cortexdb repair ./data --dry-run",
        "cortexdb repair ./data --best-effort",
        "cortexdb wal-dump ./data",
        "cortexdb wal-truncate ./data",
        "cortexdb manifest-validate ./data",
    ),
    "ops_gates": (
        "make operations-runbook-check",
        "make service-manager-smoke-check",
        "make deployment-upgrade-check",
        "make observability-check",
        "make migration-compatibility-check",
        "make storage-soak-history-check",
    ),
    "incident_response": (
        "database_busy",
        "invalid_tenant",
        "audit ./audit/http.jsonl",
        "restore from the latest validated backup",
    ),
    "boundaries": (
        "Single-node model first",
        "Production multi-node is experimental",
        "24-hour soak",
        "KMS-backed backup custody",
    ),
}

LINKED_DOCS = (
    "docs/INSTALL.md",
    "docs/SYSTEMD.md",
    "docs/LAUNCHD.md",
    "docs/BACKUP_RESTORE.md",
    "docs/RPO_RTO.md",
    "docs/UPGRADE_MIGRATION.md",
    "docs/UPGRADE_ROLLBACK.md",
    "docs/METRICS.md",
    "docs/OBSERVABILITY_ALERTS.md",
    "docs/FAILURE_SCENARIOS.md",
    "docs/CLI.md",
    "docs/API.md",
)


def validate(path: Path) -> dict[str, object]:
    failures: list[str] = []
    text = path.read_text(encoding="utf-8")
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        missing = [marker for marker in markers if marker not in text]
        checks[name] = not missing
        for marker in missing:
            failures.append(f"{path}: missing {marker!r} for {name}")
    for linked in LINKED_DOCS:
        linked_name = Path(linked).name
        if linked not in text and linked_name not in text:
            failures.append(f"{path}: missing link to {linked}")
    makefile = Path("Makefile").read_text(encoding="utf-8")
    if "operations-runbook-check:" not in makefile:
        failures.append("Makefile: missing operations-runbook-check target")
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "runbook": str(path),
        "checks": checks,
        "linked_docs_checked": list(LINKED_DOCS),
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runbook", default="docs/OPERATIONS.md")
    parser.add_argument("--report", default="target/operations-runbook/report.json")
    args = parser.parse_args()
    report = validate(Path(args.runbook))
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}")
        return 1
    print(f"operations runbook check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
