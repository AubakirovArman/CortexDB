#!/usr/bin/env python3
"""Run previous-release database and backup upgrade evidence checks."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


CURRENT_MARKER_CELL_ID = 62_999


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


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise RuntimeError(f"{path} must contain a JSON object")
    return value


def validate_db(repo: Path, db: Path) -> dict[str, Any]:
    return json.loads(run([str(cli(repo)), "--json", "validate", str(db)], repo))


def stats_db(repo: Path, db: Path) -> dict[str, Any]:
    return json.loads(run([str(cli(repo)), "--json", "stats", str(db)], repo))


def get_cell(repo: Path, db: Path, cell_id: int) -> str:
    return run([str(cli(repo)), "get", str(db), str(cell_id)], repo).rstrip("\n")


def put_cell(repo: Path, db: Path, cell_id: int, payload: str) -> None:
    run([str(cli(repo)), "put", str(db), str(cell_id), payload], repo)


def verify_expected_cells(repo: Path, db: Path, expected: list[dict[str, Any]]) -> int:
    verified = 0
    for cell in expected:
        cell_id = int(cell["cell_id"])
        expected_payload = str(cell["payload"])
        actual = get_cell(repo, db, cell_id)
        if actual != expected_payload:
            raise AssertionError(f"cell {cell_id} payload mismatch")
        verified += 1
    return verified


def copy_fixture_database(source: Path, target: Path) -> None:
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source, target)
    lock = target / "db.lock"
    if lock.exists():
        lock.unlink()


def clean_path(path: Path) -> None:
    if path.exists():
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink()


def marker_payload(from_release: str, to_release: str) -> str:
    return (
        "scope=migration:upgrade\n"
        "status=ready\n"
        f"from={from_release}\n"
        f"to={to_release}\n"
        "upgrade marker written by current binary"
    )


def restore_backup(repo: Path, backup: Path, target: Path) -> None:
    clean_path(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    run([str(cli(repo)), "restore", str(backup), str(target)], repo)


def backup_database(repo: Path, db: Path, backup: Path) -> None:
    clean_path(backup)
    backup.parent.mkdir(parents=True, exist_ok=True)
    run([str(cli(repo)), "backup", str(db), str(backup)], repo)


def run_entry(repo: Path, root: Path, entry: dict[str, Any]) -> dict[str, Any]:
    from_release = str(entry["from"])
    to_release = str(entry["to"])
    fixture_path = repo / str(entry["fixture"])
    fixture = load_json(fixture_path)
    backup_path = repo / str(entry["backup_path"])
    expected = fixture.get("expected_cells", [])
    if not expected:
        raise RuntimeError(f"{fixture_path} has no expected_cells")

    case_root = root / f"{from_release.replace('.', '_')}_to_{to_release.replace('.', '_')}"
    clean_path(case_root)
    case_root.mkdir(parents=True, exist_ok=True)
    restored_from_backup = case_root / "restored-from-backup"
    direct_database = case_root / "direct-database"
    upgraded_backup = case_root / "upgraded-backup"
    restored_after_upgrade = case_root / "restored-after-upgrade"

    restore_backup(repo, backup_path, restored_from_backup)
    restored_validation = validate_db(repo, restored_from_backup)
    restored_verified = verify_expected_cells(repo, restored_from_backup, expected)

    copy_fixture_database(backup_path, direct_database)
    direct_validation = validate_db(repo, direct_database)
    direct_verified = verify_expected_cells(repo, direct_database, expected)

    marker = marker_payload(from_release, to_release)
    put_cell(repo, direct_database, CURRENT_MARKER_CELL_ID, marker)
    run([str(cli(repo)), "flush", str(direct_database)], repo)
    run([str(cli(repo)), "compact", str(direct_database)], repo)
    upgraded_validation = validate_db(repo, direct_database)
    upgraded_stats = stats_db(repo, direct_database)
    verify_expected_cells(repo, direct_database, expected)
    if get_cell(repo, direct_database, CURRENT_MARKER_CELL_ID) != marker:
        raise AssertionError("upgrade marker cell mismatch after current write")

    backup_database(repo, direct_database, upgraded_backup)
    restore_backup(repo, upgraded_backup, restored_after_upgrade)
    restored_upgrade_validation = validate_db(repo, restored_after_upgrade)
    restored_upgrade_verified = verify_expected_cells(repo, restored_after_upgrade, expected)
    if get_cell(repo, restored_after_upgrade, CURRENT_MARKER_CELL_ID) != marker:
        raise AssertionError("upgrade marker cell mismatch after upgraded backup restore")

    return {
        "from": from_release,
        "to": to_release,
        "fixture": str(fixture_path),
        "backup_path": str(backup_path),
        "restored_from_backup": {
            "path": str(restored_from_backup),
            "validation_ok": bool(restored_validation.get("ok")),
            "cells_verified": restored_verified,
        },
        "direct_database_open": {
            "path": str(direct_database),
            "validation_ok": bool(direct_validation.get("ok")),
            "cells_verified": direct_verified,
        },
        "current_binary_write": {
            "cell_id": CURRENT_MARKER_CELL_ID,
            "validation_ok": bool(upgraded_validation.get("ok")),
            "current_seq": upgraded_stats.get("current_seq"),
        },
        "post_upgrade_backup_restore": {
            "backup_path": str(upgraded_backup),
            "restore_path": str(restored_after_upgrade),
            "validation_ok": bool(restored_upgrade_validation.get("ok")),
            "cells_verified": restored_upgrade_verified + 1,
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", default="fixtures/migration/compatibility_matrix_v1.json")
    parser.add_argument("--root", default="target/migration-upgrade-matrix-v2")
    parser.add_argument("--report", default="target/migration-upgrade-matrix-v2/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = repo_root()
    root = repo / args.root
    report_path = repo / args.report
    clean_path(root)
    root.mkdir(parents=True, exist_ok=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    started_at = utc_now()
    try:
        run(["cargo", "build", "-p", "cortex-cli", "--bin", "cortexdb"], repo)
        matrix = load_json(repo / args.matrix)
        entries = matrix.get("release_compatibility_matrix", [])
        if not entries:
            raise RuntimeError("release_compatibility_matrix is empty")
        results = [run_entry(repo, root, entry) for entry in entries]
    except Exception as error:
        print(f"migration upgrade matrix v2 check failed: {error}", file=sys.stderr)
        return 1
    status = (
        "passed"
        if all(
            item["restored_from_backup"]["validation_ok"]
            and item["direct_database_open"]["validation_ok"]
            and item["current_binary_write"]["validation_ok"]
            and item["post_upgrade_backup_restore"]["validation_ok"]
            for item in results
        )
        else "failed"
    )
    report = {
        "schema_version": 1,
        "status": status,
        "started_at": started_at,
        "finished_at": utc_now(),
        "matrix": args.matrix,
        "results": results,
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if status != "passed":
        print(f"migration upgrade matrix v2 check failed: {report_path}", file=sys.stderr)
        return 1
    print(f"migration upgrade matrix v2 check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
