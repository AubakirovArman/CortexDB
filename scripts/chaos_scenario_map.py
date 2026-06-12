#!/usr/bin/env python3
"""Emit the CortexDB crash/chaos scenario inventory.

The inventory is intentionally explicit instead of scraped from test names.
E11 uses it as the shared map that prevents adding new shutdown/crash tests
without knowing which existing harness already covers that failure mode.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


SCENARIO_GROUPS = [
    {
        "id": "engine_crash_matrix",
        "kind": "unit_crash_matrix",
        "command": "cargo test -p cortex-engine --test crash_matrix",
        "source": "crates/cortex-engine/tests/crash_matrix.rs",
        "scenarios": [
            "interrupted_checkpoint_orphan_bundle_ignored",
            "corrupt_manifest_tmp_after_checkpoint_removed_on_open",
            "missing_manifest_with_persisted_segments_fails_closed",
            "partial_manifest_fails_closed",
            "interrupted_compact_before_segment_write_replays_wal_tail",
            "interrupted_compact_after_segment_write_without_manifest_ignored",
            "interrupted_compact_without_manifest_switch_keeps_old_snapshot",
            "compact_after_manifest_update_preserves_retired_until_gc",
        ],
    },
    {
        "id": "engine_fault_injection",
        "kind": "deterministic_fault_injection",
        "command": "cargo test -p cortex-engine --test crash_consistency_fault_injection",
        "source": "crates/cortex-engine/tests/crash_consistency_fault_injection.rs",
        "scenarios": [
            "clean_checkpoint",
            "orphan_segment_before_manifest",
            "orphan_bundle_before_manifest",
            "checkpoint_manifest_before_wal_truncate",
            "checkpoint_tmp_leftovers",
            "compact_manifest_before_wal_truncate",
            "partial_wal_tail",
            "partial_bundle_before_manifest",
            "corrupt_orphan_before_manifest",
            "compact_bundle_before_manifest",
            "published_torn_checkpoint_files_fail_closed_or_validate_bad",
        ],
    },
    {
        "id": "http_chaos_restart",
        "kind": "process_chaos",
        "command": "make chaos-restart-check",
        "source": "scripts/chaos_restart_check.py",
        "scenarios": [
            "http_put_acknowledged_cell",
            "http_flush_checkpoint",
            "http_compact_full_snapshot",
            "sigkill_restart_repair_readback",
            "sigterm_restart_readback",
            "post_kill_unlock_repair_validate",
        ],
    },
    {
        "id": "storage_soak",
        "kind": "repeatable_soak",
        "command": "make storage-soak-check",
        "source": "scripts/storage_soak_check.py",
        "scenarios": [
            "repeated_write_flush_compact_validate_cycles",
            "kill_delay_process_interruption",
            "amplification_and_compaction_pressure_report",
        ],
    },
    {
        "id": "graceful_shutdown",
        "kind": "shutdown_partial",
        "command": "make chaos-restart-check",
        "source": "EPIC-E11",
        "scenarios": [
            "sigterm_drains_inflight_requests_pending",
            "server_shutdown_latency_bound",
        ],
    },
]


def scenario_index() -> dict[str, list[str]]:
    index: dict[str, list[str]] = {}
    for group in SCENARIO_GROUPS:
        for scenario in group["scenarios"]:
            index.setdefault(str(scenario), []).append(str(group["id"]))
    return index


def build_report() -> dict:
    index = scenario_index()
    duplicates = {
        scenario: groups for scenario, groups in sorted(index.items()) if len(groups) > 1
    }
    gaps = sorted(
        scenario
        for group in SCENARIO_GROUPS
        for scenario in group["scenarios"]
        if str(scenario).endswith("_pending")
    )
    return {
        "schema_version": "cortexdb.chaos_scenario_map.v1",
        "status": "partial" if gaps else "ok",
        "groups": SCENARIO_GROUPS,
        "group_count": len(SCENARIO_GROUPS),
        "scenario_count": sum(len(group["scenarios"]) for group in SCENARIO_GROUPS),
        "duplicate_scenarios": duplicates,
        "known_gaps": gaps,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="target/chaos-scenario-map/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    report = build_report()
    with output.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"chaos scenario map written to {output}")
    print(
        "groups={group_count} scenarios={scenario_count} gaps={gaps}".format(
            group_count=report["group_count"],
            scenario_count=report["scenario_count"],
            gaps=",".join(report["known_gaps"]) or "none",
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
