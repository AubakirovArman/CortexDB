#!/usr/bin/env python3
"""Validate SDK release metadata, lifecycle policy, and package hygiene."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


FORBIDDEN_TRACKED_PATTERNS = (
    re.compile(r"^sdk/python/.*\.whl$"),
    re.compile(r"^sdk/python/dist/"),
    re.compile(r"^sdk/python/build/"),
    re.compile(r"^sdk/python/.*\.egg-info/"),
    re.compile(r"^sdk/python/\.pytest_cache/"),
    re.compile(r"^sdk/typescript/node_modules/"),
    re.compile(r"^sdk/typescript/.*\.tgz$"),
)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(read_text(path))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def parse_root_version(repo: Path) -> str:
    text = read_text(repo / "Cargo.toml")
    match = re.search(r"(?m)^version = \"([^\"]+)\"", text)
    if not match:
        raise ValueError("Cargo.toml: workspace version not found")
    return match.group(1)


def parse_python_version(repo: Path) -> str:
    text = read_text(repo / "sdk/python/pyproject.toml")
    match = re.search(r"(?m)^version = \"([^\"]+)\"", text)
    if not match:
        raise ValueError("sdk/python/pyproject.toml: version not found")
    return match.group(1)


def pep440_version_for_workspace(workspace_version: str) -> str:
    """Return the Python distribution spelling for the canonical workspace version."""
    match = re.fullmatch(r"(\d+\.\d+\.\d+)-beta\.(\d+)", workspace_version)
    if match:
        return f"{match.group(1)}b{match.group(2)}"
    return workspace_version


def parse_package_name(repo: Path, path: str) -> str:
    text = read_text(repo / path)
    match = re.search(r"(?m)^name = \"([^\"]+)\"", text)
    if not match:
        raise ValueError(f"{path}: package name not found")
    return match.group(1)


def parse_openapi_version(repo: Path) -> str:
    text = read_text(repo / "docs/openapi.yaml")
    match = re.search(r"(?m)^\s*version:\s*([^\s]+)", text)
    if not match:
        raise ValueError("docs/openapi.yaml: info.version not found")
    return match.group(1).strip("'\"")


def tracked_files(repo: Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def workflow_contains(workflow: str, needle: str) -> None:
    if needle not in workflow:
        raise ValueError(f"sdk-release.yml: missing {needle!r}")


def workflow_count_at_least(workflow: str, needle: str, minimum: int) -> None:
    count = workflow.count(needle)
    if count < minimum:
        raise ValueError(f"sdk-release.yml: expected at least {minimum} occurrences of {needle!r}, found {count}")


def validate_manifest(repo: Path, errors: list[str]) -> dict[str, Any]:
    path = repo / "sdk/release-manifest.json"
    try:
        manifest = load_json(path)
        if manifest.get("schema_version") != 1:
            errors.append("sdk/release-manifest.json: schema_version must be 1")
        version_policy = manifest.get("version_policy")
        if not isinstance(version_policy, dict):
            errors.append("sdk/release-manifest.json: version_policy missing")
        else:
            for field in (
                "package_versions_must_match_workspace",
                "python_prerelease_uses_pep440_spelling",
                "openapi_version_must_start_with_workspace_version",
                "tag_must_match_workspace_version",
            ):
                if version_policy.get(field) is not True:
                    errors.append(f"sdk/release-manifest.json: version_policy.{field} must be true")
        publish_policy = manifest.get("publish_policy")
        if not isinstance(publish_policy, dict):
            errors.append("sdk/release-manifest.json: publish_policy missing")
        else:
            expected_policy = {
                "manual_only": True,
                "requires_tag_ref": True,
                "requires_explicit_publish_input": True,
            }
            for field, expected in expected_policy.items():
                if publish_policy.get(field) is not expected:
                    errors.append(f"sdk/release-manifest.json: publish_policy.{field} must be {expected}")
            if publish_policy.get("tag_prefix") != "v":
                errors.append("sdk/release-manifest.json: publish_policy.tag_prefix must be 'v'")
            if publish_policy.get("environment") != "sdk-release":
                errors.append("sdk/release-manifest.json: publish_policy.environment must be 'sdk-release'")
        registry_gate = manifest.get("registry_gate")
        if not isinstance(registry_gate, dict):
            errors.append("sdk/release-manifest.json: registry_gate missing")
        else:
            if registry_gate.get("command") != "make sdk-registry-gate-check":
                errors.append("sdk/release-manifest.json: registry_gate.command mismatch")
            if registry_gate.get("report") != "target/sdk-registry-gate/report.json":
                errors.append("sdk/release-manifest.json: registry_gate.report mismatch")
            if registry_gate.get("requires_manual_approval") is not True:
                errors.append("sdk/release-manifest.json: registry_gate.requires_manual_approval must be true")
            if registry_gate.get("does_not_claim_publication_without_release_job") is not True:
                errors.append(
                    "sdk/release-manifest.json: registry_gate must forbid publication claims without release job"
                )
        deprecation_policy = manifest.get("deprecation_policy")
        if not isinstance(deprecation_policy, dict):
            errors.append("sdk/release-manifest.json: deprecation_policy missing")
        elif deprecation_policy.get("document") != "docs/archive/SDK_DEPRECATION_POLICY.md":
            errors.append("sdk/release-manifest.json: deprecation_policy.document mismatch")
        packages = manifest.get("packages")
        if not isinstance(packages, list) or len(packages) != 3:
            errors.append("sdk/release-manifest.json: expected exactly 3 packages")
            return manifest
        required_languages = {"rust", "python", "typescript"}
        languages = {item.get("language") for item in packages if isinstance(item, dict)}
        if languages != required_languages:
            errors.append(f"sdk/release-manifest.json: package languages mismatch: {sorted(languages)}")
        for item in packages:
            if not isinstance(item, dict):
                errors.append("sdk/release-manifest.json: package entry must be object")
                continue
            for field in ("language", "name", "manifest", "registry", "publish_command", "dry_run_command"):
                if not isinstance(item.get(field), str) or not item[field]:
                    errors.append(f"sdk/release-manifest.json: {item.get('language', '?')}.{field} missing")
            manifest_path = item.get("manifest")
            if isinstance(manifest_path, str) and not (repo / manifest_path).exists():
                errors.append(f"sdk/release-manifest.json: missing package manifest {manifest_path}")
    except Exception as exc:
        errors.append(str(exc))
        return {}
    return manifest


def validate_versions(repo: Path, errors: list[str]) -> str:
    root_version = parse_root_version(repo)
    python_expected = pep440_version_for_workspace(root_version)
    versions = {
        "python": parse_python_version(repo),
        "typescript": load_json(repo / "sdk/typescript/package.json").get("version"),
        "rust": root_version,
    }
    for language, version in versions.items():
        expected = python_expected if language == "python" else root_version
        if version != expected:
            errors.append(f"{language} SDK version {version!r} != expected release version {expected!r}")
    openapi_version = parse_openapi_version(repo)
    if not openapi_version.startswith(root_version):
        errors.append(f"OpenAPI version {openapi_version!r} does not start with workspace version {root_version!r}")
    return root_version


def validate_package_metadata(repo: Path, manifest: dict[str, Any], errors: list[str]) -> None:
    expected_names = {
        "rust": parse_package_name(repo, "crates/cortex-sdk/Cargo.toml"),
        "python": parse_package_name(repo, "sdk/python/pyproject.toml"),
        "typescript": str(load_json(repo / "sdk/typescript/package.json").get("name")),
    }
    packages = manifest.get("packages", [])
    if not isinstance(packages, list):
        return
    for item in packages:
        if not isinstance(item, dict):
            continue
        language = item.get("language")
        if language in expected_names and item.get("name") != expected_names[language]:
            errors.append(
                f"sdk/release-manifest.json: {language} package name {item.get('name')!r} "
                f"!= manifest name {expected_names[language]!r}"
            )
    ts = load_json(repo / "sdk/typescript/package.json")
    for field in ("main", "module", "types", "exports", "files"):
        if field not in ts:
            errors.append(f"sdk/typescript/package.json: missing {field}")
    if "cortexdb-client.cjs" not in ts.get("files", []):
        errors.append("sdk/typescript/package.json: cjs build missing from files")


def validate_workflow(repo: Path, errors: list[str]) -> None:
    workflow = read_text(repo / ".github/workflows/sdk-release.yml")
    for needle in (
        "workflow_dispatch",
        "inputs.publish",
        "startsWith(github.ref, 'refs/tags/v')",
        "cargo publish -p cortex-sdk --dry-run",
        "python3 scripts/check_sdk_release_contract.py --enforce-github-ref",
        "python3 scripts/check_sdk_deprecation_policy.py",
        "python -m build sdk/python --wheel",
        "npm pack --dry-run",
        "environment: sdk-release",
        "pypa/gh-action-pypi-publish",
        "npm publish --access public --provenance",
        "cargo publish -p cortex-sdk",
    ):
        try:
            workflow_contains(workflow, needle)
        except ValueError as exc:
            errors.append(str(exc))
    try:
        workflow_count_at_least(workflow, 'node-version: "24"', 2)
    except ValueError as exc:
        errors.append(str(exc))


SDK_RELEASE_DOC = Path("docs/archive/SDK_RELEASE.md")


def validate_docs(repo: Path, root_version: str, errors: list[str]) -> None:
    sdk_release = read_text(repo / SDK_RELEASE_DOC)
    for phrase in (
        "manual-only",
        "publish=true",
        "tag beginning with `v`",
        "protected `sdk-release` environment",
        "tag version must match the workspace version",
        "version bump",
    ):
        if phrase not in sdk_release:
            errors.append(f"{SDK_RELEASE_DOC}: missing {phrase!r}")
    changelog = read_text(repo / "CHANGELOG.md")
    if "## Unreleased" not in changelog:
        errors.append("CHANGELOG.md: missing Unreleased section")
    if f"v{root_version}" not in changelog:
        errors.append(f"CHANGELOG.md: missing release entry for v{root_version}")
    if "SDK" not in changelog:
        errors.append("CHANGELOG.md: missing SDK notes")
    result = subprocess.run(["python3", "scripts/check_sdk_deprecation_policy.py"], cwd=repo, check=False)
    if result.returncode != 0:
        errors.append("scripts/check_sdk_deprecation_policy.py failed")


def validate_tracked_hygiene(repo: Path, errors: list[str]) -> None:
    for path in tracked_files(repo):
        for pattern in FORBIDDEN_TRACKED_PATTERNS:
            if pattern.search(path):
                errors.append(f"tracked generated SDK artifact is forbidden: {path}")


def validate_github_ref(repo: Path, root_version: str, errors: list[str]) -> None:
    ref = subprocess.run(
        ["git", "describe", "--tags", "--exact-match"],
        cwd=repo,
        check=False,
        capture_output=True,
        text=True,
    ).stdout.strip()
    github_ref = os.environ.get("GITHUB_REF", "").strip()
    if github_ref:
        if not github_ref.startswith("refs/tags/"):
            errors.append(f"GITHUB_REF must be a tag for SDK publish, got {github_ref!r}")
            return
        ref = github_ref.removeprefix("refs/tags/")
    if not ref:
        errors.append("SDK publish ref check requires an exact git tag or GITHUB_REF=refs/tags/<tag>")
        return
    pattern = re.compile(rf"^v{re.escape(root_version)}(?:[-.][A-Za-z0-9][A-Za-z0-9._-]*)?$")
    if not pattern.match(ref):
        errors.append(f"SDK publish tag {ref!r} must match workspace version {root_version!r}")


def main() -> int:
    enforce_github_ref = "--enforce-github-ref" in sys.argv[1:]
    repo = Path(__file__).resolve().parent.parent
    errors: list[str] = []
    manifest = validate_manifest(repo, errors)
    root_version = validate_versions(repo, errors)
    validate_package_metadata(repo, manifest, errors)
    validate_workflow(repo, errors)
    validate_docs(repo, root_version, errors)
    validate_tracked_hygiene(repo, errors)
    if enforce_github_ref:
        validate_github_ref(repo, root_version, errors)
    if errors:
        print("SDK RELEASE CONTRACT CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        return 1
    print("OK: SDK release contract is consistent.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
