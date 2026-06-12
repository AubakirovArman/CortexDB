#!/usr/bin/env python3
"""Validate the upgrade/rollback CLI flow contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-cli/src/cli.rs": [
        "UpgradeCommand",
        "Prepare",
        "Validate",
        "Rollback",
        "Migrate",
    ],
    "crates/cortex-cli/src/cli_upgrade.rs": [
        "upgrade_prepare",
        "ready_for_offline_upgrade",
        "migrate_preflight",
        "ready_for_offline_migration",
        "upgrade_validate",
        "validated_after_upgrade",
        "upgrade_rollback",
        "rollback_restored_and_validated",
    ],
    "crates/cortex-cli/src/tests.rs": [
        "upgrade_prepare_validate_and_rollback_flow",
        "upgrade_prepare_json_reports_next_commands",
        "migrate_preflight_creates_backup_drill_and_preserves_data",
    ],
    "docs/CLI.md": [
        "upgrade prepare",
        "upgrade validate",
        "upgrade rollback",
        "migrate <path> <backup_path> <drill_restore_path>",
    ],
    "docs/archive/UPGRADE_ROLLBACK.md": [
        "cortexdb upgrade prepare",
        "cortexdb upgrade validate",
        "cortexdb upgrade rollback",
        "cortexdb migrate",
    ],
    "docs/OPERATIONS_RUNBOOK_V1.md": [
        "cortexdb upgrade prepare",
        "cortexdb upgrade validate",
        "cortexdb upgrade rollback",
    ],
    "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md": [
        "Epic 128. Upgrade/Rollback CLI Flow",
        "cortexdb migrate",
        "target/upgrade-rollback-cli-flow/report.json",
    ],
}


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/upgrade-rollback-cli-flow/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    checked: list[str] = []

    for path, markers in REQUIRED_MARKERS.items():
        checked.append(path)
        if not Path(path).is_file():
            failures.append(f"missing {path}")
            continue
        text = read(path)
        for marker in markers:
            if marker not in text:
                failures.append(f"{path}: missing {marker!r}")

    report = {
        "schema_version": 1,
        "status": "failed" if failures else "passed",
        "checked": checked,
        "flow": {
            "prepare": "validate source, backup, restore drill",
            "validate": "validate upgraded database",
            "rollback": "dry-run restore, restore backup, validate rollback target",
        },
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"upgrade rollback cli flow check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"upgrade rollback cli flow check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
