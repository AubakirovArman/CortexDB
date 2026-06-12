#!/usr/bin/env python3
"""Run and validate the flagship demo output."""

from __future__ import annotations

import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    max_seconds = float(os.environ.get("CORTEXDB_DEMO_MAX_SECONDS", "60"))
    started = time.perf_counter()
    result = subprocess.run(
        ["./examples/demo/investment_projects/run.sh"],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    elapsed = time.perf_counter() - started
    output = result.stdout
    required = [
        "Finance agent",
        "HR agent denied as expected: ScopeNotReadable",
        "mixed evidence",
        "1.2B KZT",
        "1.4B KZT",
        "CortexDB Demo Completed Successfully",
    ]
    missing = [marker for marker in required if marker not in output]
    if missing:
        raise AssertionError(f"demo output missing markers: {missing}")
    if elapsed > max_seconds:
        raise AssertionError(f"demo took {elapsed:.1f}s, above {max_seconds:.1f}s")
    print(f"flagship demo ok elapsed={elapsed:.1f}s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
