#!/usr/bin/env python3
"""Validate the upgrade/rollback CLI flow contract and runtime drill."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path


REQUIRED_MARKERS = {
    "crates/cortex-cli/src/cli/args/commands/subcommands.rs": [
        "UpgradeCommand",
        "Prepare",
        "Validate",
        "Rollback",
    ],
    "crates/cortex-cli/src/cli/args/commands.rs": [
        "Migrate",
    ],
    "crates/cortex-cli/src/cli/dispatch/upgrade_flow.rs": [
        "UpgradeCommand::Prepare",
        "UpgradeCommand::Validate",
        "UpgradeCommand::Rollback",
        "upgrade::migrate",
    ],
    "crates/cortex-cli/src/cli_upgrade.rs": [
        "upgrade_prepare",
        "ready_for_offline_upgrade",
        "migrate_offline",
        "offline_migration_completed",
        "upgrade_validate",
        "validated_after_upgrade",
        "upgrade_rollback",
        "rollback_restored_and_validated",
    ],
    "crates/cortex-cli/src/tests/migration.rs": [
        "upgrade_prepare_validate_and_rollback_flow",
        "upgrade_prepare_json_reports_next_commands",
        "migrate_offline_creates_backup_drill_rewrites_and_preserves_data",
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
    "docs/archive/OPERATIONS_RUNBOOK_V1.md": [
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
    parser.add_argument("--cortexdb-bin", default="target/debug/cortexdb")
    parser.add_argument("--skip-runtime-drill", action="store_true")
    return parser.parse_args()


def run_command(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    return {
        "command": command,
        "returncode": completed.returncode,
        "stdout": completed.stdout.strip(),
        "stderr": completed.stderr.strip(),
    }


def require_success(command: list[str], failures: list[str]) -> dict[str, object]:
    result = run_command(command)
    if result["returncode"] != 0:
        failures.append(
            f"command failed ({result['returncode']}): {' '.join(command)} :: {result['stderr']}"
        )
    return result


def ensure_cortexdb_binary(path: Path, failures: list[str]) -> None:
    if path.is_file():
        return
    result = require_success(["cargo", "build", "-p", "cortex-cli"], failures)
    if result["returncode"] == 0 and not path.is_file():
        failures.append(f"cargo build passed but {path} was not created")


def run_runtime_drill(cortexdb_bin: Path, report_path: Path, failures: list[str]) -> dict[str, object]:
    ensure_cortexdb_binary(cortexdb_bin, failures)
    if failures:
        return {"status": "skipped", "reason": "binary unavailable"}

    root = report_path.parent / "runtime-drill"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)

    source = root / "source"
    backup = root / "pre-upgrade-backup"
    drill = root / "pre-upgrade-drill"
    rollback = root / "rollback"
    payload = "upgrade rollback release candidate payload"

    commands = [
        [str(cortexdb_bin), "put", str(source), "9001", payload],
        [str(cortexdb_bin), "flush", str(source)],
        [
            str(cortexdb_bin),
            "upgrade",
            "prepare",
            str(source),
            str(backup),
            str(drill),
        ],
        [str(cortexdb_bin), "backup-verify", str(backup)],
        [str(cortexdb_bin), "upgrade", "validate", str(source)],
        [str(cortexdb_bin), "upgrade", "rollback", str(backup), str(rollback)],
        [str(cortexdb_bin), "validate", str(rollback)],
        [str(cortexdb_bin), "get", str(rollback), "9001"],
    ]
    results = [require_success(command, failures) for command in commands]

    if results and results[-1]["stdout"] != payload:
        failures.append("rollback payload mismatch after restore")

    expected_markers = {
        "prepare": (2, ["phase=upgrade_prepare", "status=ready_for_offline_upgrade"]),
        "backup_verify": (3, ["backup_ok=true", "checksum_manifest_present=true"]),
        "validate": (4, ["phase=upgrade_validate", "status=validated_after_upgrade"]),
        "rollback": (5, ["phase=upgrade_rollback", "status=rollback_restored_and_validated"]),
    }
    for name, (index, markers) in expected_markers.items():
        stdout = str(results[index]["stdout"]) if index < len(results) else ""
        for marker in markers:
            if marker not in stdout:
                failures.append(f"runtime {name}: missing marker {marker!r}")

    return {
        "status": "failed" if failures else "passed",
        "root": str(root),
        "source_path": str(source),
        "backup_path": str(backup),
        "drill_restore_path": str(drill),
        "rollback_path": str(rollback),
        "commands": results,
    }


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    checked: list[str] = []
    report_path = Path(args.report)

    for path, markers in REQUIRED_MARKERS.items():
        checked.append(path)
        if not Path(path).is_file():
            failures.append(f"missing {path}")
            continue
        text = read(path)
        for marker in markers:
            if marker not in text:
                failures.append(f"{path}: missing {marker!r}")

    runtime_drill = (
        {"status": "skipped", "reason": "--skip-runtime-drill"}
        if args.skip_runtime_drill
        else run_runtime_drill(Path(args.cortexdb_bin), report_path, failures)
    )

    report = {
        "schema_version": 1,
        "status": "failed" if failures else "passed",
        "checked": checked,
        "flow": {
            "prepare": "validate source, backup, restore drill",
            "validate": "validate upgraded database",
            "rollback": "dry-run restore, restore backup, validate rollback target",
        },
        "runtime_drill": runtime_drill,
        "failures": failures,
    }
    output = report_path
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
