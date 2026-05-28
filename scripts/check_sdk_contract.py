#!/usr/bin/env python3
"""SDK contract drift guard.

Runs live-server smoke tests for Python and TypeScript SDKs.
Fails if either SDK cannot decode real server responses.
"""

import subprocess
import sys
from pathlib import Path


def run_python_smoke(repo: Path) -> bool:
    result = subprocess.run(
        [sys.executable, str(repo / "scripts/sdk_smoke_test.py")],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("PYTHON SDK SMOKE TEST FAILED:")
        print(result.stdout)
        print(result.stderr)
        return False
    print("OK: Python SDK smoke test")
    return True


def run_typescript_smoke(repo: Path) -> bool:
    result = subprocess.run(
        ["npx", "tsx", str(repo / "scripts/sdk_ts_smoke_test.mjs")],
        capture_output=True,
        text=True,
        cwd=repo,
    )
    if result.returncode != 0:
        print("TYPESCRIPT SDK SMOKE TEST FAILED:")
        print(result.stdout)
        print(result.stderr)
        return False
    print("OK: TypeScript SDK smoke test")
    return True


def main() -> int:
    repo = Path(__file__).parent.parent
    ok = True
    ok = run_python_smoke(repo) and ok
    ok = run_typescript_smoke(repo) and ok
    if ok:
        print("\nAll SDK contract checks passed.")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
