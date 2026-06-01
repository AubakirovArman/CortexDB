#!/usr/bin/env python3
"""Run repeatable local storage durability soak evidence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from storage_soak_lib import SoakOptions, run_storage_soak


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/storage-soak")
    parser.add_argument("--report", default="target/storage-soak/report.json")
    parser.add_argument("--cycles", type=int, default=3)
    parser.add_argument("--cells-per-cycle", type=int, default=5)
    parser.add_argument("--kill-delay-ms", type=int, default=15)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        report = run_storage_soak(
            SoakOptions(
                root=args.root,
                report=args.report,
                cycles=args.cycles,
                cells_per_cycle=args.cells_per_cycle,
                kill_delay_ms=args.kill_delay_ms,
            )
        )
    except Exception as error:
        print(f"storage soak check failed: {error}", file=sys.stderr)
        return 1

    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        print(f"storage soak check failed: {report_path}", file=sys.stderr)
        return 1
    print(f"storage soak check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
