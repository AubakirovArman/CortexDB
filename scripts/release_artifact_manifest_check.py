#!/usr/bin/env python3
"""Build and validate a formal CortexDB release artifact manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import Any

REQUIRED_REPORTS = [
    ("sdk_e2e_release", "target/sdk-e2e-release/report.json"),
    ("sdk_release_artifacts", "target/sdk-release-artifacts/report.json"),
    ("sdk_registry_gate", "target/sdk-registry-gate/report.json"),
    ("context_pack_quality", "target/context-pack-quality/report.json"),
    ("verification_quality", "target/verification-quality/report.json"),
    ("retrieval_quality", "target/retrieval-quality/report.json"),
    ("retrieval_beta", "target/retrieval-quality/beta-report.json"),
    ("binary_platform_matrix", "target/binary-platform-matrix/report.json"),
    ("install_script", "target/install-script/report.json"),
]

OPTIONAL_ARTIFACTS = [
    ("retrieval_dashboard", "target/retrieval-quality/dashboard.html"),
    ("sdk_examples_archive", "target/sdk-release-artifacts/cortexdb-sdk-examples-0.1.0.tar.gz"),
]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_entry(repo: Path, kind: str, relative: str, *, required: bool = True) -> dict[str, Any] | None:
    path = repo / relative
    if not path.is_file():
        if required:
            raise FileNotFoundError(relative)
        return None
    return {
        "kind": kind,
        "path": relative,
        "size_bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected object")
    return value


def git_value(repo: Path, command: list[str]) -> str:
    result = subprocess.run(
        ["git", *command],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def openapi_version(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    match = re.search(r"(?m)^\s*version:\s*['\"]?([^'\"\n]+)", text)
    if not match:
        raise ValueError("docs/openapi.yaml: info.version not found")
    return match.group(1).strip()


def verify_sidecar_checksum(archive: Path) -> dict[str, Any]:
    sidecar = archive.with_suffix(archive.suffix + ".sha256")
    if not sidecar.is_file():
        raise FileNotFoundError(sidecar)
    expected = sidecar.read_text(encoding="utf-8").split()[0]
    actual = sha256_file(archive)
    if expected != actual:
        raise ValueError(f"{sidecar}: checksum mismatch")
    return {
        "path": str(sidecar),
        "sha256": sha256_file(sidecar),
        "archive_sha256": actual,
    }


def binary_package_manifest(archive: Path) -> dict[str, Any]:
    with tarfile.open(archive, "r:gz") as tar:
        names = tar.getnames()
        roots = {Path(name).parts[0] for name in names if Path(name).parts}
        if len(roots) != 1:
            raise ValueError("binary archive must contain exactly one root")
        root = next(iter(roots))
        manifest_name = f"{root}/package_manifest.json"
        member = tar.extractfile(manifest_name)
        if member is None:
            raise ValueError("binary archive missing package_manifest.json")
        manifest = json.loads(member.read().decode("utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("binary package_manifest.json: unsupported schema")
    return {
        "package_id": manifest.get("package_id"),
        "version": manifest.get("version"),
        "platform": manifest.get("platform"),
        "file_count": len(manifest.get("files", [])),
        "binaries": manifest.get("binaries", []),
    }


def report_entry(repo: Path, name: str, relative: str) -> dict[str, Any]:
    path = repo / relative
    data = read_json(path)
    status = data.get("status")
    if status != "passed":
        raise ValueError(f"{relative}: expected status=passed, got {status!r}")
    entry = file_entry(repo, f"report:{name}", relative)
    assert entry is not None
    entry["status"] = status
    entry["schema_version"] = data.get("schema_version")
    return entry


def build_manifest(args: argparse.Namespace) -> dict[str, Any]:
    repo = Path(__file__).resolve().parent.parent
    binary_archive = (repo / args.binary_archive).resolve()
    if not binary_archive.is_file():
        raise FileNotFoundError(binary_archive)
    sidecar = verify_sidecar_checksum(binary_archive)
    openapi_path = repo / "docs/openapi.yaml"

    artifacts: list[dict[str, Any]] = []
    binary_relative = str(binary_archive.relative_to(repo))
    binary_entry = file_entry(repo, "binary_archive", binary_relative)
    assert binary_entry is not None
    binary_entry["sidecar"] = sidecar
    binary_entry["package_manifest"] = binary_package_manifest(binary_archive)
    artifacts.append(binary_entry)
    sidecar_entry = file_entry(repo, "binary_archive_sha256", str(Path(binary_relative + ".sha256")))
    assert sidecar_entry is not None
    artifacts.append(sidecar_entry)

    for name, relative in REQUIRED_REPORTS:
        artifacts.append(report_entry(repo, name, relative))
    for kind, relative in OPTIONAL_ARTIFACTS:
        entry = file_entry(repo, kind, relative, required=False)
        if entry is not None:
            artifacts.append(entry)

    return {
        "schema_version": "cortexdb.release_artifact_manifest.v1",
        "version": args.version,
        "git": {
            "commit": git_value(repo, ["rev-parse", "HEAD"]),
            "branch": git_value(repo, ["rev-parse", "--abbrev-ref", "HEAD"]),
            "dirty": bool(git_value(repo, ["status", "--short"])),
        },
        "openapi": {
            "path": "docs/openapi.yaml",
            "version": openapi_version(openapi_path),
            "sha256": sha256_file(openapi_path),
        },
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
        "required_reports": [name for name, _ in REQUIRED_REPORTS],
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", default="dev")
    parser.add_argument("--binary-archive", default="target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz")
    parser.add_argument("--manifest", default="target/release-artifact-manifest/manifest.json")
    parser.add_argument("--report", default="target/release-artifact-manifest/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = Path(__file__).resolve().parent.parent
    failures: list[str] = []
    manifest: dict[str, Any] | None = None
    try:
        manifest = build_manifest(args)
        manifest_path = repo / args.manifest
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except Exception as error:  # noqa: BLE001 - release gate writes structured failure.
        failures.append(str(error))

    report = {
        "schema_version": "cortexdb.release_artifact_manifest_report.v1",
        "status": "passed" if not failures else "failed",
        "manifest": args.manifest,
        "artifact_count": manifest.get("artifact_count", 0) if manifest else 0,
        "failures": failures,
    }
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"release artifact manifest check passed: {repo / args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
