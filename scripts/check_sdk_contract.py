#!/usr/bin/env python3
"""SDK contract drift guard.

Runs live-server smoke tests for Python, TypeScript, and Rust SDKs.
Fails if any SDK cannot decode real server responses.
"""

import subprocess
import sys
import os
from pathlib import Path


def ensure_server_binary(repo: Path) -> Path | None:
    result = subprocess.run(
        ["cargo", "build", "-p", "cortex-server"],
        capture_output=True,
        text=True,
        cwd=repo,
    )
    if result.returncode != 0:
        print("CORTEX SERVER BUILD FAILED:")
        print(result.stdout)
        print(result.stderr)
        return None
    print("OK: cortex-server binary")
    return repo / "target/debug/cortex-server"


def smoke_env(server_binary: Path) -> dict[str, str]:
    return {**os.environ, "CORTEXDB_SERVER_BIN": str(server_binary)}


def run_python_smoke(repo: Path, server_binary: Path) -> bool:
    result = subprocess.run(
        [sys.executable, str(repo / "scripts/sdk_smoke_test.py")],
        capture_output=True,
        text=True,
        env=smoke_env(server_binary),
    )
    if result.returncode != 0:
        print("PYTHON SDK SMOKE TEST FAILED:")
        print(result.stdout)
        print(result.stderr)
        return False
    print("OK: Python SDK smoke test")
    return True


def run_typescript_smoke(repo: Path, server_binary: Path) -> bool:
    result = subprocess.run(
        ["npx", "tsx", str(repo / "scripts/sdk_ts_smoke_test.mjs")],
        capture_output=True,
        text=True,
        cwd=repo,
        env=smoke_env(server_binary),
    )
    if result.returncode != 0:
        print("TYPESCRIPT SDK SMOKE TEST FAILED:")
        print(result.stdout)
        print(result.stderr)
        return False
    print("OK: TypeScript SDK smoke test")
    return True


def run_rust_smoke(repo: Path, server_binary: Path) -> bool:
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "cortexdb-sdk", "--example", "live_contract"],
        capture_output=True,
        text=True,
        cwd=repo,
        env=smoke_env(server_binary),
    )
    if result.returncode != 0:
        print("RUST SDK SMOKE TEST FAILED:")
        print(result.stdout)
        print(result.stderr)
        return False
    print("OK: Rust SDK smoke test")
    return True


def main() -> int:
    repo = Path(__file__).parent.parent
    ok = True
    server_binary = ensure_server_binary(repo)
    if server_binary is None:
        ok = False
    else:
        ok = run_python_smoke(repo, server_binary) and ok
        ok = run_typescript_smoke(repo, server_binary) and ok
        ok = run_rust_smoke(repo, server_binary) and ok
    if ok:
        print("\nAll SDK contract checks passed.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
