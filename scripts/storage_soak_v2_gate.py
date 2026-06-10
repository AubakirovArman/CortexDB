#!/usr/bin/env python3
"""Validate Storage Soak History v2 evidence thresholds."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REQUIRED_KILL_PHASES = {"checkpoint", "compact", "wal_replay", "restore"}


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(path)
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def throughput(entry: dict[str, Any]) -> float:
    seconds = float(entry.get("duration_seconds", 0) or 0)
    cells = float(entry.get("cells_written", 0) or 0)
    return 0.0 if seconds <= 0 else cells / seconds


def trend_failures(entries: list[dict[str, Any]], min_ratio: float) -> list[str]:
    if len(entries) < 2:
        return []
    previous = throughput(entries[-2])
    current = throughput(entries[-1])
    if previous <= 0:
        return []
    ratio = current / previous
    if ratio < min_ratio:
        return [f"latest throughput ratio {ratio:.4f} below threshold {min_ratio:.4f}"]
    return []


def validate(report: dict[str, Any], args: argparse.Namespace) -> list[str]:
    failures: list[str] = []
    total_seconds = int(report.get("total_duration_seconds", 0) or 0)
    total_cycles = int(report.get("total_cycles", 0) or 0)
    total_cells = int(report.get("total_cells_written", 0) or 0)
    entries = report.get("entries", [])
    if not isinstance(entries, list):
        entries = []

    require(report.get("status") == "passed", "history report status is not passed", failures)
    require(total_seconds >= int(args.min_hours * 3600), f"duration below {args.min_hours} hours", failures)
    require(total_cycles >= args.min_cycles, f"total_cycles below {args.min_cycles}", failures)
    require(total_cells >= args.min_cells, f"total_cells_written below {args.min_cells}", failures)
    avg_cells = 0.0 if total_cycles <= 0 else total_cells / total_cycles
    require(
        avg_cells >= args.min_avg_cells_per_cycle,
        f"average cells per cycle {avg_cells:.2f} below {args.min_avg_cells_per_cycle}",
        failures,
    )

    phase_union: set[str] = set()
    for entry in entries:
        if isinstance(entry, dict):
            phases = entry.get("kill_injection_phases", [])
            if isinstance(phases, list):
                phase_union.update(str(phase) for phase in phases)
    missing = sorted(REQUIRED_KILL_PHASES - phase_union)
    require(not missing, f"missing kill injection phases: {missing}", failures)
    failures.extend(trend_failures([e for e in entries if isinstance(e, dict)], args.min_throughput_ratio))

    require(
        "max_space_amplification_q16" in report,
        "missing space amplification evidence",
        failures,
    )
    require(
        "max_write_amplification_q16" in report,
        "missing write amplification evidence",
        failures,
    )
    require(
        "max_compaction_pressure_q16" in report,
        "missing compaction pressure evidence",
        failures,
    )
    max_space_amp = int(report.get("max_space_amplification_q16", 0) or 0)
    max_write_amp = int(report.get("max_write_amplification_q16", 0) or 0)
    max_pressure = int(report.get("max_compaction_pressure_q16", 0) or 0)
    require(
        max_space_amp <= args.max_space_amplification_q16,
        f"max space amplification q16 {max_space_amp} above {args.max_space_amplification_q16}",
        failures,
    )
    require(
        max_write_amp <= args.max_write_amplification_q16,
        f"max write amplification q16 {max_write_amp} above {args.max_write_amplification_q16}",
        failures,
    )
    require(
        max_pressure <= args.max_compaction_pressure_q16,
        f"max compaction pressure q16 {max_pressure} above {args.max_compaction_pressure_q16}",
        failures,
    )
    return failures


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/storage-soak-history-v2/report.json")
    parser.add_argument("--output", default="target/storage-soak-history-v2/v2-gate.json")
    parser.add_argument("--min-hours", type=float, default=72.0)
    parser.add_argument("--min-cycles", type=int, default=1)
    parser.add_argument("--min-cells", type=int, default=1)
    parser.add_argument("--min-avg-cells-per-cycle", type=float, default=100.0)
    parser.add_argument("--min-throughput-ratio", type=float, default=0.75)
    parser.add_argument("--max-space-amplification-q16", type=int, default=1024 * 65536)
    parser.add_argument("--max-write-amplification-q16", type=int, default=1024 * 65536)
    parser.add_argument("--max-compaction-pressure-q16", type=int, default=65536)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = load_json(Path(args.report))
        failures = validate(report, args)
    except Exception as error:  # noqa: BLE001 - gate writes structured failure report.
        report = {}
        failures = [str(error)]
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": "cortexdb.storage_soak_v2_gate.v1",
        "status": "passed" if not failures else "failed",
        "report": args.report,
        "min_hours": args.min_hours,
        "min_avg_cells_per_cycle": args.min_avg_cells_per_cycle,
        "min_throughput_ratio": args.min_throughput_ratio,
        "max_space_amplification_q16": args.max_space_amplification_q16,
        "max_write_amplification_q16": args.max_write_amplification_q16,
        "max_compaction_pressure_q16": args.max_compaction_pressure_q16,
        "observed_max_space_amplification_q16": int(
            report.get("max_space_amplification_q16", 0) or 0
        ),
        "observed_max_write_amplification_q16": int(
            report.get("max_write_amplification_q16", 0) or 0
        ),
        "observed_max_compaction_pressure_q16": int(
            report.get("max_compaction_pressure_q16", 0) or 0
        ),
        "failures": failures,
    }
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"storage soak v2 gate report: {output}")
    for failure in failures:
        print(f"failure: {failure}", file=sys.stderr)
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
