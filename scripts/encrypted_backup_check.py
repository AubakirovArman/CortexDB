#!/usr/bin/env python3
"""Run encrypted-backup MVP evidence and write a release report."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any


PASSPHRASE_ENV = "CORTEXDB_BACKUP_PASSPHRASE"
PASSPHRASE = "correct horse battery staple for cortexdb"
WRONG_PASSPHRASE = "wrong horse battery staple for cortexdb"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/encrypted-backup")
    parser.add_argument("--report", default="target/encrypted-backup/report.json")
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


def ensure_cli(cli_bin: Path, root: Path) -> Path:
    if not cli_bin.is_absolute():
        cli_bin = root / cli_bin
    if cli_bin.exists():
        return cli_bin
    run_command(["cargo", "build", "-q", "-p", "cortex-cli"], root)
    return cli_bin


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


def corrupt_copy(source: Path, target: Path) -> None:
    data = bytearray(source.read_bytes())
    if not data:
        raise SystemExit(f"empty archive cannot be corrupted: {source}")
    data[-1] ^= 0x7F
    target.write_bytes(data)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    root = repo_root()
    evidence_root = Path(args.root)
    report_path = Path(args.report)
    shutil.rmtree(evidence_root, ignore_errors=True)
    evidence_root.mkdir(parents=True, exist_ok=True)
    cli = ensure_cli(Path(args.cli_bin), root)

    db = evidence_root / "db"
    archive = evidence_root / "backup.cdbenc"
    restore = evidence_root / "restore"
    wrong_target = evidence_root / "wrong-passphrase-target"
    corrupt_archive = evidence_root / "backup-corrupt.cdbenc"
    corrupt_target = evidence_root / "corrupt-target"
    checkpoint_payload = "scope=ops\nstatus=ready\nencrypted checkpointed payload"
    wal_payload = "scope=ops\nstatus=ready\nencrypted wal tail payload"

    started = time.time()
    run_command([str(cli), "put", str(db), "1", checkpoint_payload], root)
    run_command([str(cli), "flush", str(db)], root)
    run_command([str(cli), "put", str(db), "2", wal_payload], root)

    backup, backup_ms = timed_command(
        [
            str(cli),
            "backup-encrypted",
            str(db),
            str(archive),
            "--passphrase-env",
            PASSPHRASE_ENV,
        ],
        root,
        {PASSPHRASE_ENV: PASSPHRASE},
    )
    raw_archive = archive.read_bytes()
    plaintext_hidden = (
        checkpoint_payload.encode() not in raw_archive
        and wal_payload.encode() not in raw_archive
    )

    restore_result, restore_ms = timed_command(
        [
            str(cli),
            "restore-encrypted",
            str(archive),
            str(restore),
            "--passphrase-env",
            PASSPHRASE_ENV,
        ],
        root,
        {PASSPHRASE_ENV: PASSPHRASE},
    )
    validate = run_command([str(cli), "validate", str(restore)], root)
    restored_checkpoint = run_command([str(cli), "get", str(restore), "1"], root)
    restored_wal = run_command([str(cli), "get", str(restore), "2"], root)

    wrong = run_command(
        [
            str(cli),
            "restore-encrypted",
            str(archive),
            str(wrong_target),
            "--passphrase-env",
            PASSPHRASE_ENV,
        ],
        root,
        {PASSPHRASE_ENV: WRONG_PASSPHRASE},
        expect_success=False,
    )
    corrupt_copy(archive, corrupt_archive)
    corrupt = run_command(
        [
            str(cli),
            "restore-encrypted",
            str(corrupt_archive),
            str(corrupt_target),
            "--passphrase-env",
            PASSPHRASE_ENV,
        ],
        root,
        {PASSPHRASE_ENV: PASSPHRASE},
        expect_success=False,
    )

    errors: list[str] = []
    if not plaintext_hidden:
        errors.append("archive contains plaintext payload bytes")
    if restored_checkpoint.stdout.strip() != checkpoint_payload:
        errors.append("checkpointed payload did not restore")
    if restored_wal.stdout.strip() != wal_payload:
        errors.append("WAL-tail payload did not restore")
    if wrong.returncode == 0 or wrong_target.exists():
        errors.append("wrong passphrase did not fail safely")
    if corrupt.returncode == 0 or corrupt_target.exists():
        errors.append("corrupt archive did not fail safely")

    return {
        "status": "ok" if not errors else "failed",
        "git_sha": git_sha(root),
        "generated_unix_ms": int(time.time() * 1000),
        "duration_ms": int((time.time() - started) * 1000),
        "archive_path": str(archive),
        "report_path": str(report_path),
        "backup_duration_ms": backup_ms,
        "restore_duration_ms": restore_ms,
        "backup_output": backup.stdout.strip(),
        "restore_output": restore_result.stdout.strip(),
        "validate_output": validate.stdout.strip(),
        "plaintext_hidden": plaintext_hidden,
        "wrong_passphrase_rejected": wrong.returncode != 0 and not wrong_target.exists(),
        "corrupt_ciphertext_rejected": corrupt.returncode != 0 and not corrupt_target.exists(),
        "passphrase_source": PASSPHRASE_ENV,
        "boundary": "local passphrase archive MVP, not KMS-backed or compliance-certified encryption",
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
        raise SystemExit("encrypted backup check failed: " + "; ".join(report["errors"]))
    print(f"encrypted backup evidence written to {output}")


if __name__ == "__main__":
    main()
