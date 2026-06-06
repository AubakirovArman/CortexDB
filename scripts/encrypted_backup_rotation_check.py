#!/usr/bin/env python3
"""Run encrypted-backup passphrase rotation evidence."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any


OLD_ENV = "CORTEXDB_OLD_BACKUP_PASSPHRASE"
NEW_ENV = "CORTEXDB_NEW_BACKUP_PASSPHRASE"
OLD_PASSPHRASE = "old local backup passphrase for cortexdb rotation"
NEW_PASSPHRASE = "new local backup passphrase for cortexdb rotation"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/encrypted-backup-rotation")
    parser.add_argument("--report", default="target/encrypted-backup-rotation/report.json")
    parser.add_argument("--cli-bin", default="target/debug/cortexdb")
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def git_sha(root: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def run_command(
    args: list[str],
    cwd: Path,
    env: dict[str, str] | None = None,
    expect_success: bool = True,
) -> subprocess.CompletedProcess[str]:
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    result = subprocess.run(
        args,
        cwd=cwd,
        env=merged_env,
        capture_output=True,
        text=True,
        check=False,
    )
    if expect_success and result.returncode != 0:
        raise SystemExit(
            f"command failed ({result.returncode}): {' '.join(args)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def timed_command(
    args: list[str], cwd: Path, env: dict[str, str] | None = None
) -> tuple[subprocess.CompletedProcess[str], int]:
    start = time.perf_counter_ns()
    result = run_command(args, cwd, env)
    return result, (time.perf_counter_ns() - start) // 1_000_000


def ensure_cli(cli_bin: Path, root: Path) -> Path:
    if not cli_bin.is_absolute():
        cli_bin = root / cli_bin
    if cli_bin.exists():
        return cli_bin
    run_command(["cargo", "build", "-q", "-p", "cortex-cli"], root)
    return cli_bin


def restore_encrypted(
    cli: Path,
    archive: Path,
    target: Path,
    env_name: str,
    passphrase: str,
    root: Path,
    expect_success: bool,
) -> subprocess.CompletedProcess[str]:
    return run_command(
        [
            str(cli),
            "restore-encrypted",
            str(archive),
            str(target),
            "--passphrase-env",
            env_name,
        ],
        root,
        {env_name: passphrase},
        expect_success=expect_success,
    )


def backup_encrypted(
    cli: Path,
    db: Path,
    archive: Path,
    env_name: str,
    passphrase: str,
    root: Path,
) -> tuple[subprocess.CompletedProcess[str], int]:
    return timed_command(
        [
            str(cli),
            "backup-encrypted",
            str(db),
            str(archive),
            "--passphrase-env",
            env_name,
        ],
        root,
        {env_name: passphrase},
    )


def read_cell(cli: Path, db: Path, cell_id: str, root: Path) -> str:
    return run_command([str(cli), "get", str(db), cell_id], root).stdout.strip()


def plaintext_hidden(archive: Path, payloads: list[str]) -> bool:
    raw = archive.read_bytes()
    return all(payload.encode() not in raw for payload in payloads)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = repo_root()
    evidence_root = Path(args.root)
    report_path = Path(args.report)
    shutil.rmtree(evidence_root, ignore_errors=True)
    evidence_root.mkdir(parents=True, exist_ok=True)
    cli = ensure_cli(Path(args.cli_bin), root)

    db = evidence_root / "db"
    old_archive = evidence_root / "backup-old.cdbenc"
    new_archive = evidence_root / "backup-new.cdbenc"
    old_restore = evidence_root / "restore-old"
    new_restore = evidence_root / "restore-new"
    old_with_new_target = evidence_root / "old-with-new-target"
    new_with_old_target = evidence_root / "new-with-old-target"
    old_payload = "scope=ops\nstatus=ready\nrotation old payload"
    new_payload = "scope=ops\nstatus=ready\nrotation new payload"

    started = time.time()
    run_command([str(cli), "put", str(db), "10", old_payload], root)
    run_command([str(cli), "flush", str(db)], root)
    old_backup, old_backup_ms = backup_encrypted(
        cli, db, old_archive, OLD_ENV, OLD_PASSPHRASE, root
    )

    run_command([str(cli), "put", str(db), "11", new_payload], root)
    new_backup, new_backup_ms = backup_encrypted(
        cli, db, new_archive, NEW_ENV, NEW_PASSPHRASE, root
    )

    old_restore_result, old_restore_ms = timed_command(
        [
            str(cli),
            "restore-encrypted",
            str(old_archive),
            str(old_restore),
            "--passphrase-env",
            OLD_ENV,
        ],
        root,
        {OLD_ENV: OLD_PASSPHRASE},
    )
    new_restore_result, new_restore_ms = timed_command(
        [
            str(cli),
            "restore-encrypted",
            str(new_archive),
            str(new_restore),
            "--passphrase-env",
            NEW_ENV,
        ],
        root,
        {NEW_ENV: NEW_PASSPHRASE},
    )

    run_command([str(cli), "validate", str(old_restore)], root)
    run_command([str(cli), "validate", str(new_restore)], root)
    old_readback = read_cell(cli, old_restore, "10", root)
    new_readback_old = read_cell(cli, new_restore, "10", root)
    new_readback_new = read_cell(cli, new_restore, "11", root)

    old_archive_with_new = restore_encrypted(
        cli, old_archive, old_with_new_target, NEW_ENV, NEW_PASSPHRASE, root, False
    )
    new_archive_with_old = restore_encrypted(
        cli, new_archive, new_with_old_target, OLD_ENV, OLD_PASSPHRASE, root, False
    )

    errors: list[str] = []
    if old_readback != old_payload:
        errors.append("old backup did not restore with old passphrase")
    if new_readback_old != old_payload or new_readback_new != new_payload:
        errors.append("new backup did not restore current data with new passphrase")
    if old_archive_with_new.returncode == 0 or old_with_new_target.exists():
        errors.append("old backup decrypted with new passphrase")
    if new_archive_with_old.returncode == 0 or new_with_old_target.exists():
        errors.append("new backup decrypted with old passphrase")
    if not plaintext_hidden(old_archive, [old_payload]):
        errors.append("old archive contains plaintext payload")
    if not plaintext_hidden(new_archive, [old_payload, new_payload]):
        errors.append("new archive contains plaintext payload")

    return {
        "status": "ok" if not errors else "failed",
        "git_sha": git_sha(root),
        "generated_unix_ms": int(time.time() * 1000),
        "duration_ms": int((time.time() - started) * 1000),
        "old_archive_path": str(old_archive),
        "new_archive_path": str(new_archive),
        "report_path": str(report_path),
        "old_backup_duration_ms": old_backup_ms,
        "new_backup_duration_ms": new_backup_ms,
        "old_restore_duration_ms": old_restore_ms,
        "new_restore_duration_ms": new_restore_ms,
        "old_backup_output": old_backup.stdout.strip(),
        "new_backup_output": new_backup.stdout.strip(),
        "old_restore_output": old_restore_result.stdout.strip(),
        "new_restore_output": new_restore_result.stdout.strip(),
        "old_backup_decrypts_with_old_passphrase": old_readback == old_payload,
        "new_backup_decrypts_with_new_passphrase": (
            new_readback_old == old_payload and new_readback_new == new_payload
        ),
        "old_backup_rejects_new_passphrase": (
            old_archive_with_new.returncode != 0 and not old_with_new_target.exists()
        ),
        "new_backup_rejects_old_passphrase": (
            new_archive_with_old.returncode != 0 and not new_with_old_target.exists()
        ),
        "old_archive_plaintext_hidden": plaintext_hidden(old_archive, [old_payload]),
        "new_archive_plaintext_hidden": plaintext_hidden(
            new_archive, [old_payload, new_payload]
        ),
        "rotation_policy": (
            "MVP rotation creates a new encrypted backup with the new passphrase; "
            "old archives remain decryptable only with the old passphrase until retired."
        ),
        "boundary": "passphrase rotation evidence only, not KMS-backed key rotation",
        "errors": errors,
    }


def main() -> None:
    args = parse_args()
    report = build_report(args)
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    if report["status"] != "ok":
        raise SystemExit(
            "encrypted backup rotation check failed: " + "; ".join(report["errors"])
        )
    print(f"encrypted backup rotation evidence written to {output}")


if __name__ == "__main__":
    main()
