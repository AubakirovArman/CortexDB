#!/usr/bin/env python3
"""Validate local distributed-consensus evidence gates."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


GATE_MARKERS = {
    "distributed-consensus": {
        "schema": "cortexdb.consensus.distributed_gate.v1",
        "required_suites": [],
        "make_test_suites": [
            "replication_log",
            "replication_log_matching",
            "replication_commit",
            "replication_election",
            "replication_membership",
            "replication_replay_apply",
        ],
        "markers": [
            ("docs/archive/DISTRIBUTED_CONSENSUS_DESIGN.md", "Consensus State"),
            ("docs/archive/DISTRIBUTED_CONSENSUS_DESIGN.md", "Replicated Log"),
            ("docs/archive/DISTRIBUTED_CONSENSUS_DESIGN.md", "Snapshot Install"),
            ("docs/archive/DISTRIBUTED_CONSENSUS_DESIGN.md", "Membership Changes"),
            ("docs/archive/CONSENSUS_SLO.md", "Consensus is still experimental"),
            ("mk/core-security-ops.mk", "distributed-consensus-check"),
            ("crates/cortex-engine/tests/replication_log.rs", "replication_log_recovers_consensus_state"),
            ("crates/cortex-engine/tests/replication_log_matching.rs", "append_entries_truncates_conflicting_suffix"),
            ("crates/cortex-engine/tests/replication_commit.rs", "current_term_commit_indirectly_commits_prior_term_prefix"),
            ("crates/cortex-engine/tests/replication_election.rs", "higher_term_leader_replaces_previous_term_leader"),
            ("crates/cortex-engine/tests/replication_membership.rs", "joint_consensus_requires_old_and_new_majorities"),
            ("crates/cortex-engine/tests/replication_replay_apply.rs", "replay_apply_is_idempotent_after_recovery"),
        ],
    },
    "partition-soak": {
        "schema": "cortexdb.consensus.partition_soak_gate.v1",
        "required_suites": [
            "failure_injection",
            "partition_matrix",
            "consensus_hardening",
        ],
        "make_test_suites": [],
        "markers": [
            ("docs/archive/CONSENSUS_SLO.md", "repeatable split-brain/rejoin repair soak"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make consensus-partition-soak-check"),
            ("mk/core-security-ops.mk", "consensus-partition-soak-check"),
        ],
    },
    "failover-slo": {
        "schema": "cortexdb.consensus.failover_slo_gate.v1",
        "required_suites": [
            "failure_injection",
            "partition_matrix",
            "consensus_hardening",
        ],
        "make_test_suites": [],
        "markers": [
            ("docs/archive/CONSENSUS_SLO.md", "failover detection and leader replacement"),
            ("docs/archive/CONSENSUS_SLO.md", "Beta promotion requirement"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make consensus-failover-slo-check"),
            ("mk/core-security-ops.mk", "consensus-failover-slo-check"),
        ],
    },
    "rejoin": {
        "schema": "cortexdb.consensus.rejoin_gate.v1",
        "required_suites": [
            "repair_after_rejoin",
            "repair_cycle",
            "repair_worker",
            "consensus_hardening",
            "snapshot_sender",
            "snapshot_faults",
            "membership_rotation",
            "runtime",
        ],
        "make_test_suites": [],
        "markers": [
            ("docs/archive/CONSENSUS_SLO.md", "Rejoin repair"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make consensus-rejoin-check"),
            ("mk/core-security-ops.mk", "consensus-rejoin-check"),
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


def evidence_suite_names(reports: list[dict[str, Any]]) -> set[str]:
    names: set[str] = set()
    for report in reports:
        for suite in report.get("suites", []):
            if isinstance(suite, dict) and isinstance(suite.get("name"), str):
                names.add(suite["name"])
    return names


def validate(gate: str, evidence_paths: list[Path]) -> dict[str, Any]:
    spec = GATE_MARKERS[gate]
    failures: list[str] = []
    checks: dict[str, bool] = {}

    for file_name, marker in spec["markers"]:
        if marker not in read(Path(file_name)):
            failures.append(f"marker {marker!r} missing from {file_name}")
    checks["markers"] = not failures

    reports = [load_report(path) for path in evidence_paths]
    for path, report in zip(evidence_paths, reports):
        if report.get("status") not in {"ok", "passed"}:
            failures.append(f"evidence report {path} status is {report.get('status')!r}")
    checks["evidence_reports_passed"] = not any(
        report.get("status") not in {"ok", "passed"} for report in reports
    )

    required_suites = set(spec["required_suites"])
    observed_suites = evidence_suite_names(reports)
    missing_suites = sorted(required_suites.difference(observed_suites))
    if missing_suites:
        failures.append(f"missing evidence suites: {', '.join(missing_suites)}")
    checks["required_suites_present"] = not missing_suites

    total_passed = sum(int(report.get("total_passed_tests", 0)) for report in reports)
    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "production_ready": False,
        "boundary": "local consensus evidence only; no production HA claim",
        "evidence_reports": [str(path) for path in evidence_paths],
        "direct_test_suites": spec["make_test_suites"],
        "observed_suites": sorted(observed_suites),
        "required_suites": sorted(required_suites),
        "total_passed_tests_from_evidence": total_passed,
        "checks": checks,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATE_MARKERS))
    parser.add_argument("--evidence", action="append", default=[])
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    try:
        report = validate(args.gate, [Path(path) for path in args.evidence])
    except RuntimeError as error:
        print(f"consensus gate check failed: {error}", file=sys.stderr)
        return 1

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"consensus {args.gate} check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
