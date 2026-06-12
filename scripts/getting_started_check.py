#!/usr/bin/env python3
"""Run the commands documented in docs/GETTING_STARTED.md."""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DB_PATH = ROOT / "target/getting-started-demo"


def run(args: list[str]) -> str:
    result = subprocess.run(
        args,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.stdout


def run_cli(*args: str) -> str:
    return run(["cargo", "run", "-q", "-p", "cortex-cli", "--", *args])


def main() -> int:
    shutil.rmtree(DB_PATH, ignore_errors=True)

    run(["cargo", "build", "-p", "cortex-cli"])
    loaded = run_cli("load-fixture", str(DB_PATH), "examples/datasets/investment_projects")
    assert "cells_count=5" in loaded, loaded

    stats = run_cli("stats", str(DB_PATH))
    assert "cells=" in stats, stats

    search = json.loads(
        run_cli("--json", "search", str(DB_PATH), "project:investments", "Solar Plant budget")
    )
    assert search["results"], search

    context = json.loads(
        run_cli(
            "context",
            "--format",
            "json",
            str(DB_PATH),
            "project:investments",
            'RETRIEVE CONTEXT FOR TASK "Solar Plant budget evidence" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;',
        )
    )
    assert context["schema_version"] == "context_pack.v1", context
    assert context["cells"], context

    verify = json.loads(
        run_cli(
            "verify",
            "--format",
            "json",
            str(DB_PATH),
            "project:investments",
            'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;',
        )
    )
    assert verify["verdict"] == "mixed_evidence", verify
    assert verify["supporting"], verify
    assert verify["contradicting"], verify

    hr_search = json.loads(
        run_cli("--json", "search", str(DB_PATH), "agent:hr", "Solar Plant budget")
    )
    assert hr_search["results"] == [], hr_search

    print("getting started ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
