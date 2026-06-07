#!/usr/bin/env python3
"""Check the checksum-verifying install script with a throwaway package."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
from pathlib import Path


def run(command: list[str], cwd: Path) -> str:
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)
    output = result.stdout + ("\n" + result.stderr if result.stderr else "")
    if result.returncode != 0:
        raise RuntimeError(f"command failed {command}: {output.strip()}")
    return output


def write_fake_binary(path: Path, label: str) -> None:
    path.write_text(f"#!/usr/bin/env sh\nprintf '{label}\\n'\n", encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    script = repo / "scripts" / "install.sh"
    if not script.is_file():
        print("error: scripts/install.sh missing", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        bin_dir = root / "fake-bin"
        bin_dir.mkdir()
        write_fake_binary(bin_dir / "cortexdb", "cortexdb-test")
        write_fake_binary(bin_dir / "cortex-server", "cortex-server-test")

        archive = root / "cortexdb-test-linux-x86_64.tar.gz"
        run([
            "python3",
            "scripts/package_binaries.py",
            "--package-id",
            "cortexdb-test-linux-x86_64",
            "--platform",
            "linux-x86_64",
            "--version",
            "test",
            "--bin-dir",
            str(bin_dir),
            "--archive",
            str(archive),
        ], repo)

        dry_prefix = root / "dry-prefix"
        dry_output = run([str(script), str(archive), "--prefix", str(dry_prefix), "--dry-run"], repo)
        if "verified" not in dry_output:
            raise RuntimeError(f"dry run did not verify archive: {dry_output}")
        if "next steps" not in dry_output:
            raise RuntimeError(f"dry run did not print next steps: {dry_output}")
        if dry_prefix.exists():
            raise RuntimeError("dry run must not create install prefix")

        url_prefix = root / "url-prefix"
        url_output = run(
            [
                str(script),
                f"file://{archive}",
                "--sha256",
                f"file://{archive}.sha256",
                "--prefix",
                str(url_prefix),
                "--dry-run",
            ],
            repo,
        )
        if "verified" not in url_output or "next steps" not in url_output:
            raise RuntimeError(f"URL dry run did not verify archive and print next steps: {url_output}")

        prefix = root / "prefix"
        install_output = run([str(script), str(archive), "--prefix", str(prefix)], repo)
        if "installed cortexdb binaries" not in install_output:
            raise RuntimeError(f"install output missing success marker: {install_output}")
        if "next steps" not in install_output or "cortexdb validate ./data" not in install_output:
            raise RuntimeError(f"install output missing next steps: {install_output}")

        installed_cli = prefix / "bin" / "cortexdb"
        installed_server = prefix / "bin" / "cortex-server"
        if not installed_cli.is_file() or not os.access(installed_cli, os.X_OK):
            raise RuntimeError("installed cortexdb is missing or not executable")
        if not installed_server.is_file() or not os.access(installed_server, os.X_OK):
            raise RuntimeError("installed cortex-server is missing or not executable")

        cli_output = run([str(installed_cli)], repo).strip()
        server_output = run([str(installed_server)], repo).strip()
        if cli_output != "cortexdb-test" or server_output != "cortex-server-test":
            raise RuntimeError("installed binaries do not match package contents")

        corrupt_checksum = archive.with_suffix(archive.suffix + ".sha256")
        original_checksum = corrupt_checksum.read_text(encoding="utf-8")
        corrupt_checksum.write_text(original_checksum.replace("  ", "  missing-", 1), encoding="utf-8")
        failed = subprocess.run(
            [str(script), str(archive), "--prefix", str(root / "bad-prefix")],
            cwd=repo,
            capture_output=True,
            text=True,
            check=False,
        )
        if failed.returncode == 0:
            raise RuntimeError("install script accepted corrupt external checksum")
        if (root / "bad-prefix").exists():
            raise RuntimeError("failed checksum install must not create prefix")

    report = {
        "schema_version": "cortexdb.install_script_check.v1",
        "status": "passed",
        "flows": [
            "download_url",
            "external_checksum",
            "internal_checksums",
            "dry_run",
            "install",
            "next_steps",
            "corrupt_checksum_rejected",
        ],
    }
    report_path = repo / "target" / "install-script" / "report.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"install script check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
