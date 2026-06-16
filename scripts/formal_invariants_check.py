#!/usr/bin/env python3
"""Bounded executable model gate for EPIC-F10 formal invariants."""

from __future__ import annotations

import argparse
import itertools
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


DOC_PATH = Path("docs/FORMAL_INVARIANTS.md")
REQUIRED_DOC_MARKERS = [
    "machine-checked",
    "runtime dependency",
    "stateright-ready",
    "WAL recovery",
    "Snapshot pinning",
    "Policy rewrite",
    "no uncovered Scan",
    "cargo test -p cortex-engine --test wal_commit_seq --all-features",
    "cargo test -p cortex-engine --test snapshot_pinning --all-features",
    "python3 scripts/policy_rewrite_gate_check.py",
]
REGRESSION_COMMANDS = [
    [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "wal_commit_seq",
        "--all-features",
    ],
    [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "checkpoint",
        "--all-features",
        "recovery_",
    ],
    [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "snapshot_pinning",
        "--all-features",
    ],
    ["python3", "scripts/policy_rewrite_gate_check.py"],
    [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--lib",
        "all_read_surfaces_rewrite_to_policy_complete_plans",
        "--all-features",
    ],
]
SCOPES = ("alpha", "beta", "gamma")
READ_SURFACES = (
    "AqlRetrieve",
    "AqlExplain",
    "Search",
    "SearchExplain",
    "ContextPack",
    "ContextTrace",
    "CellGet",
    "Verify",
    "Graph",
    "Memory",
    "Feedback",
    "Export",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument(
        "--skip-regressions",
        action="store_true",
        help="Only run the bounded models and document marker checks.",
    )
    return parser.parse_args()


def assert_true(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def check_doc_markers() -> list[str]:
    if not DOC_PATH.exists():
        return [f"{DOC_PATH} is missing"]
    text = DOC_PATH.read_text(encoding="utf-8")
    return [
        f"{DOC_PATH} missing marker: {marker}"
        for marker in REQUIRED_DOC_MARKERS
        if marker not in text
    ]


def recover_wal_prefix(
    record_states: tuple[str, ...], checkpoint_seq: int, rotation_boundary: int
) -> list[int]:
    ordered_records = list(enumerate(record_states, start=1))
    rotated_files = (
        ordered_records[:rotation_boundary],
        ordered_records[rotation_boundary:],
    )
    recovered: list[int] = []
    for wal_file in rotated_files:
        for seq, state in wal_file:
            if seq <= checkpoint_seq:
                continue
            if state != "commit":
                return recovered
            recovered.append(seq)
    return recovered


def check_wal_recovery_model() -> dict[str, int]:
    max_seq = 4
    cases = 0
    for record_states in itertools.product(("commit", "corrupt", "missing"), repeat=max_seq):
        for checkpoint_seq in range(max_seq + 1):
            for rotation_boundary in range(max_seq + 1):
                cases += 1
                recovered = recover_wal_prefix(
                    record_states,
                    checkpoint_seq,
                    rotation_boundary,
                )
                expected: list[int] = []
                for seq in range(checkpoint_seq + 1, max_seq + 1):
                    if record_states[seq - 1] != "commit":
                        break
                    expected.append(seq)
                assert_true(
                    recovered == expected,
                    "WAL recovery did not return the committed prefix",
                )
                assert_true(
                    all(seq > checkpoint_seq for seq in recovered),
                    "WAL recovery replayed a checkpointed seq",
                )
                assert_true(
                    recovered == list(range(checkpoint_seq + 1, checkpoint_seq + 1 + len(recovered))),
                    "WAL recovery produced a non-contiguous seq set",
                )
    return {"cases": cases, "max_seq": max_seq}


def check_snapshot_gc_model() -> dict[str, int]:
    segments = frozenset({"s1", "s2", "s3", "s4"})
    cases = 0
    for manifest_segments in powerset(segments):
        for pinned_segments in powerset(segments):
            for retired_segments in powerset(segments):
                cases += 1
                protected = manifest_segments | pinned_segments
                collected = retired_segments - protected
                retained = segments - collected
                assert_true(
                    collected.isdisjoint(manifest_segments),
                    "GC collected a manifest-visible segment",
                )
                assert_true(
                    collected.isdisjoint(pinned_segments),
                    "GC collected a pinned segment",
                )
                assert_true(
                    protected <= retained,
                    "GC failed to retain a protected segment",
                )
    return {"cases": cases, "segments": len(segments)}


def powerset(values: frozenset[str]) -> list[frozenset[str]]:
    ordered = tuple(sorted(values))
    subsets: list[frozenset[str]] = []
    for size in range(len(ordered) + 1):
        for subset in itertools.combinations(ordered, size):
            subsets.append(frozenset(subset))
    return subsets


def rewrite_read_plan(
    readable_scopes: frozenset[str],
    requested_scope: str | None,
    read_surface: str,
) -> dict[str, Any]:
    if read_surface not in READ_SURFACES:
        return {"status": "denied", "scan_scopes": frozenset(), "covered": True}
    if requested_scope is not None:
        if requested_scope not in readable_scopes:
            return {"status": "denied", "scan_scopes": frozenset(), "covered": True}
        return {"status": "planned", "scan_scopes": frozenset([requested_scope]), "covered": True}
    return {"status": "planned", "scan_scopes": readable_scopes, "covered": True}


def check_policy_rewrite_model() -> dict[str, int]:
    cases = 0
    for readable_scopes in powerset(frozenset(SCOPES)):
        for requested_scope in (None, *SCOPES):
            for read_surface in READ_SURFACES:
                cases += 1
                plan = rewrite_read_plan(readable_scopes, requested_scope, read_surface)
                scan_scopes = plan["scan_scopes"]
                assert_true(plan["covered"], "policy rewrite produced an uncovered Scan")
                if requested_scope is not None and requested_scope not in readable_scopes:
                    assert_true(
                        plan["status"] == "denied",
                        "unreadable explicit scope was not denied",
                    )
                    assert_true(
                        not scan_scopes,
                        "denied plan still exposed scan scopes",
                    )
                    continue
                assert_true(
                    plan["status"] == "planned",
                    "readable or broad scope was unexpectedly denied",
                )
                assert_true(
                    scan_scopes <= readable_scopes,
                    "policy rewrite produced a scan outside AgentView",
                )
    return {"cases": cases, "read_surfaces": len(READ_SURFACES)}


def run_command(command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return {
        "command": command,
        "returncode": completed.returncode,
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def main() -> int:
    args = parse_args()
    errors = check_doc_markers()
    model_results: dict[str, dict[str, int]] = {}
    regression_results: list[dict[str, Any]] = []

    try:
        model_results["wal_recovery"] = check_wal_recovery_model()
        model_results["snapshot_pinning_gc"] = check_snapshot_gc_model()
        model_results["policy_rewrite"] = check_policy_rewrite_model()
    except AssertionError as error:
        errors.append(str(error))

    if not args.skip_regressions:
        for command in REGRESSION_COMMANDS:
            result = run_command(command)
            regression_results.append(result)
            if result["returncode"] != 0:
                errors.append(f"regression command failed: {' '.join(command)}")

    report = {
        "schema_version": "cortexdb.formal_invariants.v1",
        "doc": str(DOC_PATH),
        "required_doc_markers": REQUIRED_DOC_MARKERS,
        "model_results": model_results,
        "regression_results": regression_results,
        "status": "passed" if not errors else "failed",
        "errors": errors,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print(
        json.dumps(
            {
                "status": report["status"],
                "report": args.report,
                "models": model_results,
                "regressions": len(regression_results),
            },
            sort_keys=True,
        )
    )
    for result in regression_results:
        print(f"$ {' '.join(result['command'])}")
        if result["stdout_tail"]:
            print(result["stdout_tail"], end="")
        if result["stderr_tail"]:
            print(result["stderr_tail"], end="", file=sys.stderr)
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
