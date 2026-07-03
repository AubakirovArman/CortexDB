#!/usr/bin/env python3
"""F5.2: benchmark-score regression gate.

A committed benchmark result must never silently regress below its recorded
floor. This reads each committed benchmark report and asserts the headline metric
is at or above a committed floor (fixtures/benchmarks/floors.v1.json). A drop
below the floor fails the gate — so a ranking or pipeline change that quietly
lowers a published score is caught in CI, not after release. Dependency-free.
"""

from __future__ import annotations

import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
FLOORS = REPO / "fixtures" / "benchmarks" / "floors.v1.json"


def dig(obj, dotted: str):
    cur = obj
    for part in dotted.split("."):
        cur = cur[part]
    return cur


def main() -> int:
    report_path = None
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])

    spec = json.loads(FLOORS.read_text())
    checks = []
    ok = True
    for entry in spec["benchmarks"]:
        artifact = REPO / entry["file"]
        row = {"name": entry["name"], "metric": entry["metric"], "floor": entry["floor"]}
        try:
            actual = float(dig(json.loads(artifact.read_text()), entry["metric"]))
            row["actual"] = actual
            row["ok"] = actual >= entry["floor"] - 1e-9
        except Exception as e:  # noqa: BLE001
            row["error"] = str(e)
            row["ok"] = False
        ok = ok and row["ok"]
        checks.append(row)

    passed = ok and len(checks) > 0
    report = {
        "schema_version": "cortexdb.benchmark_floor_check.v1",
        "status": "passed" if passed else "failed",
        "checks": checks,
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    if not passed:
        print("benchmark-floor-check FAILED")
        for c in checks:
            if not c.get("ok"):
                print(f"  {c['name']} {c['metric']}: {c.get('actual', c.get('error'))} < floor {c['floor']}")
        return 1
    print(f"benchmark-floor-check passed: {len(checks)} benchmark(s) at or above their committed floor")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
