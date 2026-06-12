#!/usr/bin/env python3
"""Validate binary platform matrix docs and clean-install behavior."""

from __future__ import annotations

import argparse
import json
import socket
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
from pathlib import Path
from typing import Any


DOC_MARKERS = {
    "docs/archive/BINARY_PLATFORM_MATRIX.md": [
        "linux-x86_64",
        "linux-aarch64",
        "macos-arm64",
        "macos-x86_64",
        "Windows is unsupported",
        "Clean Install Smoke",
        "Filesystem Requirements",
        "file `fsync`",
        "atomic `rename`",
        "`db.lock`",
        "network filesystems",
        "launchd",
    ],
    "docs/archive/BINARY_RELEASES.md": [
        "Binary Platform Matrix",
        "Linux and macOS",
        "No Windows binary artifact",
    ],
    "docs/INSTALL.md": [
        "cortexdb-<version>-linux-x86_64.tar.gz",
        "cortexdb-<version>-linux-aarch64.tar.gz",
        "cortexdb-<version>-macos-arm64.tar.gz",
        "cortexdb-<version>-macos-x86_64.tar.gz",
        "Binary platform matrix",
    ],
    "docs/deployment/com.cortexdb.server.plist": [
        "/usr/local/bin/cortex-server",
        "/usr/local/var/cortexdb",
        "KeepAlive",
    ],
}


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require_markers(repo: Path) -> list[str]:
    failures: list[str] = []
    for relative, markers in DOC_MARKERS.items():
        path = repo / relative
        if not path.is_file():
            failures.append(f"missing {relative}")
            continue
        text = read(path)
        for marker in markers:
            if marker not in text:
                failures.append(f"{relative}: missing {marker!r}")
    workflow = read(repo / ".github/workflows/release.yml")
    for marker in (
        "ubuntu-latest",
        "ubuntu-24.04-arm",
        "macos-latest",
        "macos-13",
        "platform: linux-x86_64",
        "platform: linux-aarch64",
        "platform: macos-arm64",
        "platform: macos-x86_64",
        "BINARY_RELEASE_PLATFORM=\"$platform\"",
        "make binary-release-check",
        "gh release upload",
    ):
        if marker not in workflow:
            failures.append(f".github/workflows/release.yml: missing {marker!r}")
    return failures


def run(command: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)
    output = result.stdout + ("\n" + result.stderr if result.stderr else "")
    if result.returncode != 0:
        raise RuntimeError(f"command failed {command}: {output.strip()}")
    return output


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def wait_http(url: str, timeout_seconds: float = 5.0) -> None:
    deadline = time.time() + timeout_seconds
    last_error: Exception | None = None
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=0.5) as response:
                if response.status == 200:
                    return
        except Exception as error:  # noqa: BLE001 - retry until server is ready.
            last_error = error
        time.sleep(0.1)
    raise RuntimeError(f"server did not become ready at {url}: {last_error}")


def extract_archive(archive: Path, output: Path) -> Path:
    output.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar.getmembers():
            path = Path(member.name)
            if path.is_absolute() or ".." in path.parts:
                raise RuntimeError(f"unsafe archive member {member.name}")
        tar.extractall(output)
    roots = [path for path in output.iterdir() if path.is_dir()]
    if len(roots) != 1:
        raise RuntimeError("binary archive must contain exactly one root directory")
    return roots[0]


def clean_install_smoke(repo: Path, archive: Path) -> dict[str, Any]:
    if not archive.is_file():
        raise FileNotFoundError(archive)
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        package_root = extract_archive(archive, root / "extract")
        cortexdb = package_root / "bin" / "cortexdb"
        server = package_root / "bin" / "cortex-server"
        data = root / "db"
        restored = root / "restored"
        backup = root / "backup"

        run([str(cortexdb), "version"])
        run([str(cortexdb), "load-fixture", str(data), str(repo / "examples/datasets/investment_projects")])
        run([str(cortexdb), "validate", str(data)])
        search_output = run([str(cortexdb), "search", str(data), "project:investments", "Solar"])
        if "results=" not in search_output and "cell_id" not in search_output:
            raise RuntimeError(f"search did not return expected output: {search_output}")
        run([str(cortexdb), "backup", str(data), str(backup)])
        run([str(cortexdb), "restore", str(backup), str(restored)])
        run([str(cortexdb), "validate", str(restored)])

        port = free_port()
        process = subprocess.Popen(
            [str(server), str(restored), f"127.0.0.1:{port}"],
            cwd=repo,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            wait_http(f"http://127.0.0.1:{port}/v1/health")
            request = urllib.request.Request(
                f"http://127.0.0.1:{port}/v1/search?scope=project%3Ainvestments&q=Solar",
                data=b"",
                method="POST",
            )
            with urllib.request.urlopen(request, timeout=2) as response:
                body = response.read().decode("utf-8")
            if "results" not in body:
                raise RuntimeError(f"server query missing results: {body}")
        finally:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)

    return {
        "archive": str(archive),
        "fixture": "examples/datasets/investment_projects",
        "flows": ["version", "load_fixture", "validate", "search", "backup", "restore", "server_health", "server_query"],
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", default="target/release-artifacts/cortexdb-local.tar.gz")
    parser.add_argument("--report", default="target/binary-platform-matrix/report.json")
    parser.add_argument("--skip-smoke", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = Path(__file__).resolve().parent.parent
    failures = require_markers(repo)
    smoke: dict[str, Any] | None = None
    if not args.skip_smoke and not failures:
        try:
            smoke = clean_install_smoke(repo, repo / args.archive)
        except Exception as error:  # noqa: BLE001 - release gate writes structured failure.
            failures.append(str(error))
    report = {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "platforms": {
            "supported": ["linux-x86_64", "linux-aarch64", "macos-arm64", "macos-x86_64"],
            "unsupported": ["windows"],
        },
        "release_workflow": {
            "path": ".github/workflows/release.yml",
            "platforms": ["linux-x86_64", "linux-aarch64", "macos-arm64", "macos-x86_64"],
            "upload": "gh release upload",
        },
        "filesystem_requirements": [
            "local_posix_style_filesystem",
            "wal_file_fsync",
            "atomic_same_directory_rename",
            "parent_directory_durability",
            "exclusive_lock_file_create",
            "regular_executable_files",
        ],
        "filesystem_warnings": [
            "network_filesystems_require_operator_validation",
            "cloud_sync_folders_are_not_recommended_for_production_like_data",
            "container_overlay_paths_require_operator_validation",
        ],
        "clean_install_smoke": smoke,
        "failures": failures,
    }
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"binary platform matrix check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
