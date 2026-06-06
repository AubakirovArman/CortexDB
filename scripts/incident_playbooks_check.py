#!/usr/bin/env python3
"""Validate that CortexDB incident playbooks cover required local incidents."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


PLAYBOOKS: dict[str, tuple[str, ...]] = {
    "corrupted_storage": (
        "## Playbook 1. Corrupted Storage",
        "Trigger examples:",
        "Triage:",
        "Containment:",
        "Recovery:",
        "Exit criteria:",
        "cortexdb validate ./data",
        "cortexdb repair ./data --dry-run",
        "cortexdb restore ./backups/latest-validated",
    ),
    "actor_busy": (
        "## Playbook 2. Actor Busy",
        "database_busy",
        "actor_queue_depth",
        "/v1/metrics",
        "Reduce caller concurrency",
        "make load-smoke-check",
    ),
    "backup_failed": (
        "## Playbook 3. Backup Failed",
        "CortexDbBackupStale",
        "CortexDbBackupEvidenceMissing",
        "cortexdb backup-drill ./data",
        "cortexdb backup-offsite-stage",
        "make backup-restore-production-pack-check",
    ),
    "auth_failure_spike": (
        "## Playbook 4. Auth Failure Spike",
        "401",
        "403",
        "cortexdb audit ./audit/http.jsonl --summary --redaction-check",
        "cortexdb auth-review",
        "make security-gate-v2-check",
    ),
    "tenant_issue": (
        "## Playbook 5. Tenant Issue",
        "invalid_tenant",
        "TENANT_NAMING_RULES.md",
        "cortexdb validate ./data --tenant",
        "make tenant-recovery-check",
        "make quota-policy-check",
    ),
}

GENERAL_MARKERS = (
    "Save evidence before changing state.",
    "make incident-playbooks-check",
    "OPERATIONS_RUNBOOK_V1.md",
)


def validate(path: Path) -> dict[str, object]:
    failures: list[str] = []
    text = path.read_text(encoding="utf-8")
    checks: dict[str, bool] = {}
    for marker in GENERAL_MARKERS:
        if marker not in text:
            failures.append(f"{path}: missing general marker {marker!r}")
    for name, markers in PLAYBOOKS.items():
        missing = [marker for marker in markers if marker not in text]
        checks[name] = not missing
        for marker in missing:
            failures.append(f"{path}: missing {marker!r} for {name}")
    makefile = Path("Makefile").read_text(encoding="utf-8")
    if "incident-playbooks-check:" not in makefile:
        failures.append("Makefile: missing incident-playbooks-check target")
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "playbooks": str(path),
        "checks": checks,
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--playbooks", default="docs/INCIDENT_PLAYBOOKS.md")
    parser.add_argument("--report", default="target/incident-playbooks/report.json")
    args = parser.parse_args()
    report = validate(Path(args.playbooks))
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}")
        return 1
    print(f"incident playbooks check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
