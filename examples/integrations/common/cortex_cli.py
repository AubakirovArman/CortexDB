"""Small stdlib CortexDB CLI wrapper used by integration examples."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_DATASET = ROOT / "examples/datasets/investment_projects"


def cortexdb_command() -> list[str]:
    configured = os.environ.get("CORTEXDB_BIN")
    if configured:
        return [configured]
    return ["cargo", "run", "-q", "-p", "cortex-cli", "--"]


def run_cortexdb(args: list[str]) -> str:
    completed = subprocess.run(
        [*cortexdb_command(), *args],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return completed.stdout.strip()


def run_json(args: list[str]) -> dict[str, Any]:
    output = run_cortexdb(args)
    return json.loads(output)


def quote_aql(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def reset_fixture_database(db_path: Path, dataset: Path = DEFAULT_DATASET) -> None:
    shutil.rmtree(db_path, ignore_errors=True)
    run_cortexdb(["load-fixture", str(db_path), str(dataset.relative_to(ROOT))])


def retrieve_context(db_path: Path, scope: str, task: str, limit: int = 10) -> dict[str, Any]:
    aql = (
        f'RETRIEVE CONTEXT FOR TASK "{quote_aql(task)}" IN BRAIN default '
        f"REQUIRE citations LIMIT {limit} CANDIDATES;"
    )
    return run_json(["context", "--format", "json", str(db_path), scope, aql])


def verify_fact(db_path: Path, scope: str, fact: str) -> dict[str, Any]:
    aql = f'VERIFY FACT "{quote_aql(fact)}" IN BRAIN default;'
    return run_json(["verify", "--format", "json", str(db_path), scope, aql])


def remember_memory(
    db_path: Path,
    scope: str,
    text: str,
    memory_type: str = "preference",
    ttl_seconds: int = 3600,
) -> dict[str, Any]:
    aql = (
        f'REMEMBER "{quote_aql(text)}" IN SCOPE {scope} '
        f"AS TYPE {memory_type} TTL {ttl_seconds} SECONDS;"
    )
    return run_json(["--json", "remember", str(db_path), scope, aql])


def citations(context_pack: dict[str, Any]) -> list[str]:
    return [
        cell.get("citation", "")
        for cell in context_pack.get("cells", [])
        if isinstance(cell, dict) and cell.get("citation")
    ]
