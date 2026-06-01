#!/usr/bin/env python3
"""Restore historical backup fixtures with the current CortexDB CLI."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def run(cmd: list[str], repo: Path) -> str:
    result = subprocess.run(
        cmd,
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({' '.join(cmd)}):\n{result.stdout}")
    return result.stdout


def cli(repo: Path) -> Path:
    return repo / "target/debug/cortexdb"


def load_fixtures(repo: Path) -> list[tuple[Path, dict[str, Any]]]:
    root = repo / "fixtures/migration/historical"
    fixtures = []
    for path in sorted(root.glob("*/fixture.json")):
        fixtures.append((path, json.loads(path.read_text(encoding="utf-8"))))
    if not fixtures:
        raise RuntimeError("no historical migration restore fixtures found")
    return fixtures


def check_fixture(repo: Path, root: Path, path: Path, fixture: dict[str, Any]) -> dict[str, Any]:
    release = fixture["release_tag"]
    backup = path.parent / fixture["backup_path"]
    restore = root / release / "restored"
    if restore.exists():
        shutil.rmtree(restore)
    restore.parent.mkdir(parents=True, exist_ok=True)
    run([str(cli(repo)), "restore", str(backup), str(restore)], repo)
    validation = json.loads(run([str(cli(repo)), "--json", "validate", str(restore)], repo))
    verified = 0
    for cell in fixture["expected_cells"]:
        output = run([str(cli(repo)), "get", str(restore), str(cell["cell_id"])], repo).rstrip("\n")
        if output != cell["payload"]:
            raise AssertionError(f"{release}: cell {cell['cell_id']} payload mismatch")
        verified += 1
    return {
        "release_tag": release,
        "fixture": str(path),
        "backup": str(backup),
        "restore": str(restore),
        "validation_ok": bool(validation.get("ok")),
        "cells_verified": verified,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/migration-historical-restore")
    parser.add_argument("--report", default="target/migration-historical-restore/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = repo_root()
    root = repo / args.root
    report_path = repo / args.report
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    started_at = utc_now()
    try:
        run(["cargo", "build", "-p", "cortex-cli", "--bin", "cortexdb"], repo)
        results = [check_fixture(repo, root, path, fixture) for path, fixture in load_fixtures(repo)]
    except Exception as error:
        print(f"historical migration restore check failed: {error}", file=sys.stderr)
        return 1
    status = "passed" if all(item["validation_ok"] for item in results) else "failed"
    report = {
        "schema_version": 1,
        "status": status,
        "started_at": started_at,
        "finished_at": utc_now(),
        "fixtures": results,
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if status != "passed":
        print(f"historical migration restore check failed: {report_path}", file=sys.stderr)
        return 1
    print(f"historical migration restore check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
