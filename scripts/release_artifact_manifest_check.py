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
    ("sdk_examples_archive", "target/sdk-release-artifacts/cortexdb-sdk-examples-0.2.0-beta.2.tar.gz"),
]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


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
    value = json.loads(read_text(path))
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
    text = read_text(path)
    match = re.search(r"(?m)^\s*version:\s*['\"]?([^'\"\n]+)", text)
    if not match:
        raise ValueError("docs/openapi.yaml: info.version not found")
    return match.group(1).strip()


def toml_version(path: Path, fallback: str | None = None) -> str:
    text = read_text(path)
    if "version.workspace = true" in text and fallback is not None:
        return fallback
    match = re.search(r"(?m)^\s*version\s*=\s*\"([^\"]+)\"", text)
    if not match:
        raise ValueError(f"{path}: version not found")
    return match.group(1)


def package_json_version(path: Path) -> str:
    value = json.loads(read_text(path))
    version = value.get("version")
    if not isinstance(version, str) or not version:
        raise ValueError(f"{path}: version not found")
    return version


def pep440_version_for_workspace(workspace_version: str) -> str:
    match = re.fullmatch(r"(\d+\.\d+\.\d+)-beta\.(\d+)", workspace_version)
    if match:
        return f"{match.group(1)}b{match.group(2)}"
    return workspace_version


def sdk_versions(repo: Path) -> dict[str, Any]:
    workspace_version = toml_version(repo / "Cargo.toml")
    python_expected = pep440_version_for_workspace(workspace_version)
    versions = {
        "workspace": workspace_version,
        "rust": {"name": "cortex-sdk", "version": toml_version(repo / "crates/cortex-sdk/Cargo.toml", workspace_version), "manifest": "crates/cortex-sdk/Cargo.toml"},
        "python": {"name": "cortexdb-client", "version": toml_version(repo / "sdk/python/pyproject.toml"), "manifest": "sdk/python/pyproject.toml"},
        "typescript": {"name": "@cortexdb/client", "version": package_json_version(repo / "sdk/typescript/package.json"), "manifest": "sdk/typescript/package.json"},
        "python_pep440_expected": python_expected,
        "source": "sdk/release-manifest.json",
    }
    expected_versions = {
        "rust": workspace_version,
        "python": python_expected,
        "typescript": workspace_version,
    }
    for language, expected in expected_versions.items():
        version = versions[language]["version"]
        if version != expected:
            raise ValueError(
                f"{language} SDK version {version!r} != expected release version {expected!r}"
            )
    return versions


def storage_format_versions(repo: Path) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    table = read_text(repo / "docs/STORAGE_FORMATS.md").splitlines()
    for line in table:
        if not line.startswith("| ") or "Magic" in line or "---" in line:
            continue
        cells = [cell.strip().strip("`") for cell in line.strip("|").split("|")]
        if len(cells) < 4:
            continue
        name, file_extension, magic, version_state = cells[:4]
        if file_extension.startswith("."):
            rows.append({"name": name, "file_extension": file_extension, "magic": magic, "version_state": version_state, "source": "docs/STORAGE_FORMATS.md"})
    if len(rows) < 7:
        raise ValueError("docs/STORAGE_FORMATS.md: expected at least 7 storage format rows")
    return rows


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


def evidence_bundle_entry(repo: Path, relative: str, required: bool) -> list[dict[str, Any]]:
    archive = repo / relative
    if not archive.is_file():
        if required:
            raise FileNotFoundError(relative)
        return []
    sidecar = verify_sidecar_checksum(archive)
    archive_entry = file_entry(repo, "release_evidence_bundle", relative)
    assert archive_entry is not None
    archive_entry["sidecar"] = sidecar
    sidecar_entry = file_entry(repo, "release_evidence_bundle_sha256", str(Path(relative + ".sha256")))
    assert sidecar_entry is not None
    return [archive_entry, sidecar_entry]


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
    return {"package_id": manifest.get("package_id"), "version": manifest.get("version"), "platform": manifest.get("platform"), "file_count": len(manifest.get("files", [])), "binaries": manifest.get("binaries", [])}


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
    artifacts.extend(evidence_bundle_entry(repo, args.evidence_bundle, args.require_evidence_bundle))

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
        "sdk_versions": sdk_versions(repo),
        "storage_format_versions": storage_format_versions(repo),
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
        "required_reports": [name for name, _ in REQUIRED_REPORTS],
    }


def requirement_coverage(manifest: dict[str, Any] | None) -> dict[str, Any]:
    if not manifest:
        return {
            "binary_checksums": False,
            "evidence_checksums": False,
            "sdk_versions": False,
            "openapi_version": False,
            "storage_format_versions": False,
        }
    artifacts = manifest.get("artifacts", [])
    artifact_kinds = {item.get("kind") for item in artifacts if isinstance(item, dict)}
    binary_archives = [
        item for item in artifacts if isinstance(item, dict) and item.get("kind") == "binary_archive"
    ]
    evidence_bundles = [
        item
        for item in artifacts
        if isinstance(item, dict) and item.get("kind") == "release_evidence_bundle"
    ]
    sdk = manifest.get("sdk_versions", {})
    openapi = manifest.get("openapi", {})
    storage_formats = manifest.get("storage_format_versions", [])
    return {
        "binary_checksums": bool(
            binary_archives
            and binary_archives[0].get("sha256")
            and binary_archives[0].get("sidecar", {}).get("archive_sha256")
            and "binary_archive_sha256" in artifact_kinds
        ),
        "evidence_checksums": bool(
            evidence_bundles
            and evidence_bundles[0].get("sha256")
            and evidence_bundles[0].get("sidecar", {}).get("archive_sha256")
            and "release_evidence_bundle_sha256" in artifact_kinds
        ),
        "sdk_versions": bool(
            isinstance(sdk, dict)
            and sdk.get("workspace")
            and sdk.get("rust", {}).get("version")
            and sdk.get("python", {}).get("version")
            and sdk.get("typescript", {}).get("version")
        ),
        "openapi_version": bool(openapi.get("version") and openapi.get("sha256")),
        "storage_format_versions": isinstance(storage_formats, list) and len(storage_formats) >= 7,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", default="dev")
    parser.add_argument("--binary-archive", default="target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz")
    parser.add_argument("--evidence-bundle", default="target/release-evidence-bundle/release-evidence.tar.gz")
    parser.add_argument("--require-evidence-bundle", action="store_true")
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
        coverage = requirement_coverage(manifest)
        required_coverage = dict(coverage)
        if not args.require_evidence_bundle:
            required_coverage.pop("evidence_checksums", None)
        missing = [name for name, covered in required_coverage.items() if not covered]
        if missing:
            raise ValueError(f"release artifact manifest missing requirement coverage: {missing}")
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
        "requirement_coverage": requirement_coverage(manifest),
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
