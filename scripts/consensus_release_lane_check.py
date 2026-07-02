#!/usr/bin/env python3
"""Validate consensus release-lane promotion with consecutive green runs."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]

RELEASE_LANE_GATES = [
    "partition-soak",
    "failover-slo",
    "rejoin",
    "failover-binder",
    "multi-agent-cluster-consistency",
    "receipt-replica-invariance",
]

WIRING_MARKERS = {
    "mk/release.mk": ["$(MAKE) consensus-release-lane-check"],
    "mk/core-security-ops.mk": [
        "consensus-release-lane-check:",
        "CONSENSUS_RELEASE_LANE_RUNS",
        "scripts/consensus_release_lane_check.py",
    ],
    "mk/vars-core.mk": [
        "CONSENSUS_RELEASE_LANE_RUNS",
        "CONSENSUS_RELEASE_LANE_REPORT",
    ],
    "mk/phony.mk": ["consensus-release-lane-check"],
    "docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md": [
        "consensus-release-lane-check",
        "N consecutive soak-green",
    ],
    "docs/STATUS.md": ["consensus-release-lane-check"],
    "docs/COMMUNITY_ROADMAP.md": ["consensus-release-lane-check"],
    "docs/SECURITY_MODEL.md": ["consensus-release-lane-check"],
}


def read_text(relative: str) -> str:
    path = ROOT / relative
    if not path.exists():
        raise RuntimeError(f"missing required file: {relative}")
    return path.read_text(encoding="utf-8")


def check_wiring() -> list[str]:
    errors: list[str] = []
    for relative, markers in WIRING_MARKERS.items():
        text = read_text(relative)
        for marker in markers:
            if marker not in text:
                errors.append(f"{relative} missing marker: {marker}")
    return errors


def run_command(command: list[str], log_path: Path) -> dict[str, Any]:
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(result.stdout, encoding="utf-8")
    return {
        "command": command,
        "exit_code": result.returncode,
        "log": str(log_path),
        "status": "passed" if result.returncode == 0 else "failed",
    }


def load_report(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def report_passed(path: Path) -> bool:
    report = load_report(path)
    return report.get("status") in {"ok", "passed"}


def run_consensus_iteration(run_root: Path, index: int) -> dict[str, Any]:
    run_dir = run_root / f"run-{index:02d}"
    if run_dir.exists():
        shutil.rmtree(run_dir)
    run_dir.mkdir(parents=True)

    partition_root = run_dir / "replication-partition"
    partition_report = partition_root / "report.json"
    lifecycle_root = run_dir / "replication-lifecycle"
    lifecycle_report = lifecycle_root / "report.json"
    reports = {
        "partition-soak": run_dir / "partition-soak.json",
        "failover-slo": run_dir / "failover-slo.json",
        "rejoin": run_dir / "rejoin.json",
        "failover-binder": run_dir / "failover-binder.json",
        "multi-agent-cluster-consistency": run_dir / "multi-agent-cluster-consistency.json",
        "receipt-replica-invariance": run_dir / "receipt-replica-invariance.json",
    }
    commands = [
        [
            "make",
            "replication-partition-check",
            f"REPLICATION_PARTITION_ROOT={partition_root}",
            f"REPLICATION_PARTITION_REPORT={partition_report}",
        ],
        [
            "python3",
            "scripts/consensus_gate_check.py",
            "--gate",
            "partition-soak",
            "--evidence",
            str(partition_report),
            "--report",
            str(reports["partition-soak"]),
        ],
        [
            "python3",
            "scripts/consensus_gate_check.py",
            "--gate",
            "failover-slo",
            "--evidence",
            str(partition_report),
            "--report",
            str(reports["failover-slo"]),
        ],
        [
            "make",
            "replication-lifecycle-check",
            f"REPLICATION_LIFECYCLE_ROOT={lifecycle_root}",
            f"REPLICATION_LIFECYCLE_REPORT={lifecycle_report}",
        ],
        [
            "python3",
            "scripts/consensus_gate_check.py",
            "--gate",
            "rejoin",
            "--evidence",
            str(partition_report),
            "--evidence",
            str(lifecycle_report),
            "--report",
            str(reports["rejoin"]),
        ],
        [
            "make",
            "consensus-failover-binder-check",
            f"CONSENSUS_FAILOVER_BINDER_REPORT={reports['failover-binder']}",
        ],
        [
            "make",
            "multi-agent-cluster-consistency-check",
            f"MULTI_AGENT_CLUSTER_CONSISTENCY_REPORT={reports['multi-agent-cluster-consistency']}",
        ],
        [
            "make",
            "receipt-replica-invariance-check",
            f"RECEIPT_REPLICA_INVARIANCE_REPORT={reports['receipt-replica-invariance']}",
        ],
    ]

    command_results = []
    for step, command in enumerate(commands, start=1):
        result = run_command(command, run_dir / "logs" / f"step-{step:02d}.log")
        command_results.append(result)
        if result["exit_code"] != 0:
            break

    report_status = {
        gate: report_passed(path) if path.exists() else False for gate, path in reports.items()
    }
    return {
        "run": index,
        "status": "passed"
        if all(result["exit_code"] == 0 for result in command_results)
        and all(report_status.values())
        else "failed",
        "commands": command_results,
        "reports": {gate: str(path) for gate, path in reports.items()},
        "report_status": report_status,
    }


def write_report(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-root", default="target/consensus/release-lane")
    parser.add_argument("--report", default="target/consensus/release-lane/report.json")
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--check-wiring-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    errors = check_wiring()
    iterations: list[dict[str, Any]] = []

    if not args.check_wiring_only and not errors:
        if args.runs < 1:
            errors.append("--runs must be at least 1")
        else:
            run_root = Path(args.run_root)
            for index in range(1, args.runs + 1):
                result = run_consensus_iteration(run_root, index)
                iterations.append(result)
                if result["status"] != "passed":
                    errors.append(f"consensus release-lane run {index} failed")
                    break

    report = {
        "schema_version": "cortexdb.consensus.release_lane_gate.v1",
        "status": "passed" if not errors else "failed",
        "release_ready": not errors,
        "production_ready": False,
        "boundary": "release-lane CI evidence only; not a live production HA claim",
        "required_gates": RELEASE_LANE_GATES,
        "runs_required": args.runs,
        "runs_passed": sum(1 for run in iterations if run["status"] == "passed"),
        "wiring_checked": sorted(WIRING_MARKERS),
        "iterations": iterations,
        "errors": errors,
    }
    write_report(Path(args.report), report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
