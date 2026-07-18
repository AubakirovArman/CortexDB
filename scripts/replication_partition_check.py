#!/usr/bin/env python3
"""Run the explicit replication partition/split-brain release gate."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from pathlib import Path


TEST_SUITES = [
    {
        "name": "failure_injection",
        "test": "replication_failure_injection",
        "coverage": [
            "minority partition cannot commit",
            "healed majority rejects stale leader",
            "idempotent replication-log replay after restart",
        ],
    },
    {
        "name": "partition_matrix",
        "test": "replication_partition_matrix",
        "coverage": [
            "five-node minority/majority partition matrix",
            "partitioned leader restart",
            "majority election rejects stale minority leader",
            "TCP snapshot transport smoke",
        ],
    },
    {
        "name": "repair_after_rejoin",
        "test": "replication_repair",
        "coverage": [
            "rejoined voter append catch-up",
            "snapshot-required lag classification",
        ],
    },
    {
        "name": "repair_cycle",
        "test": "replication_repair_cycle",
        "coverage": [
            "one-shot append repair execution",
            "snapshot handoff surfaced explicitly",
        ],
    },
    {
        "name": "repair_worker",
        "test": "replication_repair_worker",
        "coverage": [
            "bounded repair worker loop",
            "durable progress-driven repair planning",
        ],
    },
    {
        "name": "consensus_hardening",
        "test": "replication_consensus_hardening",
        "coverage": [
            "repeatable split-brain/rejoin repair soak",
            "follower lag repair snapshot threshold then idle",
            "membership rotation resume can continue to next rotation",
        ],
    },
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/replication-partition")
    parser.add_argument("--report", default="target/replication-partition/report.json")
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
        "gate": "replication_partition_split_brain",
        "suites": suites,
        "total_passed_tests": sum(
            suite["passed_tests"] or 0 for suite in suites if suite["status"] == "ok"
        ),
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"replication partition evidence written to {report_path}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
