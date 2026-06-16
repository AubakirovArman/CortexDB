#!/usr/bin/env python3
"""Install the published SDK packages from public registries in clean projects."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class PackageVersions:
    workspace: str
    rust_name: str
    python_name: str
    python_version: str
    typescript_name: str
    typescript_version: str


def read_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def package_versions() -> PackageVersions:
    root_cargo = read_toml(ROOT / "Cargo.toml")
    workspace_version = root_cargo["workspace"]["package"]["version"]
    rust = read_toml(ROOT / "crates/cortex-sdk/Cargo.toml")
    python = read_toml(ROOT / "sdk/python/pyproject.toml")
    typescript = json.loads((ROOT / "sdk/typescript/package.json").read_text(encoding="utf-8"))
    return PackageVersions(
        workspace=workspace_version,
        rust_name=rust["package"]["name"],
        python_name=python["project"]["name"],
        python_version=python["project"]["version"],
        typescript_name=typescript["name"],
        typescript_version=typescript["version"],
    )


def run(cmd: list[str], *, cwd: Path, check_name: str) -> str:
    result = subprocess.run(
        cmd,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{check_name} failed with exit {result.returncode}: {' '.join(cmd)}\n{result.stdout}"
        )
    return result.stdout


def require_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise RuntimeError(f"required tool not found on PATH: {name}")


def rust_smoke(root: Path, versions: PackageVersions) -> dict[str, Any]:
    require_tool("cargo")
    project = root / "rust"
    project.mkdir()
    run(["cargo", "init", "--bin", "--quiet", "."], cwd=project, check_name="cargo init")
    run(
        ["cargo", "add", f"{versions.rust_name}@{versions.workspace}", "--quiet"],
        cwd=project,
        check_name="cargo add",
    )
    (project / "src/main.rs").write_text(
        "\n".join(
            [
                "use cortex_sdk::CortexDbClient;",
                "",
                "fn main() {",
                '    let _client = CortexDbClient::new("http://127.0.0.1:8181");',
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    run(["cargo", "check", "--quiet"], cwd=project, check_name="cargo check")
    return {
        "language": "rust",
        "package": versions.rust_name,
        "version": versions.workspace,
        "commands": ["cargo add", "cargo check"],
        "status": "passed",
    }


def python_smoke(root: Path, versions: PackageVersions) -> dict[str, Any]:
    project = root / "python"
    project.mkdir()
    venv = project / "venv"
    run([sys.executable, "-m", "venv", str(venv)], cwd=project, check_name="python venv")
    python = venv / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
    run([str(python), "-m", "pip", "install", "--quiet", "--upgrade", "pip"], cwd=project, check_name="pip upgrade")
    run(
        [
            str(python),
            "-m",
            "pip",
            "install",
            "--quiet",
            f"{versions.python_name}=={versions.python_version}",
        ],
        cwd=project,
        check_name="pip install",
    )
    run(
        [
            str(python),
            "-c",
            "from cortexdb_client import CortexDBClient; assert CortexDBClient.__name__ == 'CortexDBClient'",
        ],
        cwd=project,
        check_name="python import",
    )
    return {
        "language": "python",
        "package": versions.python_name,
        "version": versions.python_version,
        "commands": ["pip install", "python import"],
        "status": "passed",
    }


def typescript_smoke(root: Path, versions: PackageVersions) -> dict[str, Any]:
    require_tool("npm")
    require_tool("node")
    project = root / "typescript"
    project.mkdir()
    run(["npm", "init", "-y"], cwd=project, check_name="npm init")
    run(
        ["npm", "install", "--silent", f"{versions.typescript_name}@{versions.typescript_version}"],
        cwd=project,
        check_name="npm install",
    )
    run(
        [
            "node",
            "--input-type=module",
            "-e",
            (
                f"import {{ CortexDBClient }} from '{versions.typescript_name}'; "
                "if (typeof CortexDBClient !== 'function') process.exit(1);"
            ),
        ],
        cwd=project,
        check_name="node import",
    )
    return {
        "language": "typescript",
        "package": versions.typescript_name,
        "version": versions.typescript_version,
        "commands": ["npm install", "node import"],
        "status": "passed",
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/sdk-public-registry-smoke/report.json")
    parser.add_argument("--keep-temp", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    versions = package_versions()
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    temp_root = Path(tempfile.mkdtemp(prefix="cortexdb-sdk-public-registry-"))
    checks: list[dict[str, Any]] = []
    try:
        checks.append(rust_smoke(temp_root, versions))
        checks.append(python_smoke(temp_root, versions))
        checks.append(typescript_smoke(temp_root, versions))
        report = {
            "schema_version": "cortexdb.sdk_public_registry_smoke.v1",
            "status": "passed",
            "workspace_version": versions.workspace,
            "checks": checks,
        }
        print("sdk public registry smoke passed")
        return_code = 0
    except Exception as error:  # noqa: BLE001 - release smoke reports as JSON.
        report = {
            "schema_version": "cortexdb.sdk_public_registry_smoke.v1",
            "status": "failed",
            "workspace_version": versions.workspace,
            "checks": checks,
            "error": str(error),
        }
        print(f"sdk public registry smoke failed: {error}", file=sys.stderr)
        return_code = 1
    finally:
        report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        if args.keep_temp:
            print(f"kept temp dir: {temp_root}")
        else:
            shutil.rmtree(temp_root, ignore_errors=True)
    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
