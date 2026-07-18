#!/usr/bin/env python3
"""Run replication snapshot/resync/membership lifecycle release evidence."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from pathlib import Path


TEST_SUITES = [
    {
        "name": "snapshot_sender",
        "test": "replication_snapshot_sender",
        "coverage": [
            "chunked snapshot send",
            "cumulative ACK enforcement",
            "TCP peer durable install",
        ],
    },
    {
        "name": "snapshot_faults",
        "test": "replication_snapshot_faults",
        "coverage": [
            "partial snapshot does not replace follower",
            "stale chunk rejected",
            "corrupt final chunk rejected",
        ],
    },
    {
        "name": "database_snapshot_source",
        "test": "replication_database_snapshot",
        "coverage": [
            "snapshot sourced from current database storage",
            "background repair can use database-backed snapshots",
        ],
    },
    {
        "name": "repair_background",
        "test": "replication_repair_background",
        "coverage": [
            "stoppable repair background task",
            "finite-run repair scheduling",
        ],
    },
    {
        "name": "repair_worker",
        "test": "replication_repair_worker",
        "coverage": [
            "append repair until idle",
            "snapshot-required handoff without spin",
        ],
    },
    {
        "name": "progress_store",
        "test": "replication_progress_store",
        "coverage": [
            "durable follower progress persistence",
            "progress recording transport",
            "snapshot ACK progress recording",
        ],
    },
    {
        "name": "progress_membership",
        "test": "replication_progress_membership",
        "coverage": [
            "joined voters seeded",
            "retired voters pruned",
            "joint config tracks voter union",
        ],
    },
    {
        "name": "progress_runtime",
        "test": "replication_progress_runtime",
        "coverage": [
            "default progress-recording background runtime",
            "runtime progress resumes after restart",
        ],
    },
    {
        "name": "membership",
        "test": "replication_membership",
        "coverage": [
            "membership log encoding/recovery",
            "joint consensus quorum",
            "automated join/leave rotation",
        ],
    },
    {
        "name": "membership_rotation",
        "test": "replication_membership_rotation",
        "coverage": [
            "restart resume from committed joint config",
            "uncommitted joint config rejected",
        ],
    },
    {
        "name": "runtime",
        "test": "replication_runtime",
        "coverage": [
            "topology startup",
            "committed membership before progress reconcile",
            "bad commit index rejected",
        ],
    },
    {
        "name": "cluster_config",
        "test": "replication_cluster_config",
        "coverage": [
            "durable operator topology config",
            "node-scoped replication paths",
        ],
    },
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/replication-lifecycle")
    parser.add_argument("--report", default="target/replication-lifecycle/report.json")
    return parser.parse_args()


def run_suite(repo: Path, root: Path, suite: dict[str, object]) -> dict[str, object]:
    log_path = root / f"{suite['name']}.log"
    command = [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--features",
        "experimental-replication",
        "--test",
        str(suite["test"]),
        "--",
        "--nocapture",
    ]
    result = subprocess.run(
        command,
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    log_path.write_text(result.stdout, encoding="utf-8")
    passed = parse_passed_count(result.stdout)
    suite_ok = result.returncode == 0 and passed is not None and passed > 0
    return {
        "name": suite["name"],
        "test": suite["test"],
        "command": command,
        "coverage": suite["coverage"],
        "status": "ok" if suite_ok else "failed",
        "exit_code": result.returncode,
        "passed_tests": passed,
        "log": str(log_path),
    }


def parse_passed_count(output: str) -> int | None:
    match = re.search(r"test result: ok\. ([0-9]+) passed", output)
    if not match:
        return None
    return int(match.group(1))


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[1]
    root = Path(args.root)
    report_path = Path(args.report)

    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)

    suites = [run_suite(repo, root, suite) for suite in TEST_SUITES]
    ok = all(suite["status"] == "ok" for suite in suites)
    report = {
        "status": "ok" if ok else "failed",
        "gate": "replication_snapshot_resync_membership_lifecycle",
        "suites": suites,
        "total_passed_tests": sum(
            suite["passed_tests"] or 0 for suite in suites if suite["status"] == "ok"
        ),
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"replication lifecycle evidence written to {report_path}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
