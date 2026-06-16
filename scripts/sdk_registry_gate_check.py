#!/usr/bin/env python3
"""Validate SDK registry publication gate wiring."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REQUIRED_PACKAGES = {
    "rust_api_types": {"registry": "crates.io", "publish": "cargo publish -p cortex-api-types", "dry_run": "cargo publish -p cortex-api-types --dry-run"},
    "rust": {
        "registry": "crates.io",
        "publish": "cargo publish -p cortexdb-sdk",
        "dry_run": "cargo publish -p cortexdb-sdk --dry-run",
    },
    "python": {
        "registry": "pypi",
        "publish": "pypa/gh-action-pypi-publish",
        "dry_run": "python -m build sdk/python --wheel",
    },
    "typescript": {
        "registry": "npm",
        "publish": "npm publish --access public --provenance",
        "dry_run": "npm pack --dry-run",
    },
}

REQUIRED_WORKFLOW_MARKERS = (
    "workflow_dispatch",
    "inputs.publish",
    "startsWith(github.ref, 'refs/tags/v')",
    "environment: sdk-release",
    "id-token: write",
    "pypa/gh-action-pypi-publish",
    "npm publish --access public --provenance",
    "cargo publish -p cortexdb-sdk",
    "secrets.PYPI_API_TOKEN",
    "secrets.NPM_TOKEN",
    "secrets.CARGO_REGISTRY_TOKEN",
)

REQUIRED_DOC_MARKERS = {
    "docs/archive/SDK_RELEASE.md": (
        "manual-only",
        "publish=true",
        "protected `sdk-release` environment",
        "Registry credentials are configured",
    ),
    "docs/archive/SDK_PUBLICATION_STATUS.md": (
        "public registry publication is not claimed",
        "manual `sdk-release` GitHub environment approves publication",
        "Registry credentials are configured",
    ),
}


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(read(path))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected object")
    return value


def check_manifest(repo: Path, failures: list[str]) -> dict[str, Any]:
    manifest = load_json(repo / "sdk/release-manifest.json")
    publish_policy = manifest.get("publish_policy", {})
    expected_policy = {
        "manual_only": True,
        "requires_tag_ref": True,
        "requires_explicit_publish_input": True,
        "environment": "sdk-release",
        "tag_prefix": "v",
    }
    for key, expected in expected_policy.items():
        if publish_policy.get(key) != expected:
            failures.append(f"publish_policy.{key} must be {expected!r}")

    registry_gate = manifest.get("registry_gate", {})
    if registry_gate.get("command") != "make sdk-registry-gate-check":
        failures.append("registry_gate.command must be make sdk-registry-gate-check")
    if registry_gate.get("report") != "target/sdk-registry-gate/report.json":
        failures.append("registry_gate.report must be target/sdk-registry-gate/report.json")
    if registry_gate.get("requires_manual_approval") is not True:
        failures.append("registry_gate.requires_manual_approval must be true")
    if registry_gate.get("does_not_claim_publication_without_release_job") is not True:
        failures.append("registry_gate must forbid publication claims without release job")
    credentials = registry_gate.get("requires_registry_credentials", [])
    for credential in ("PYPI_API_TOKEN", "NPM_TOKEN", "CARGO_REGISTRY_TOKEN"):
        if credential not in credentials:
            failures.append(f"registry_gate missing credential requirement {credential!r}")

    packages = manifest.get("packages", [])
    by_language = {item.get("language"): item for item in packages if isinstance(item, dict)}
    for language, expected in REQUIRED_PACKAGES.items():
        item = by_language.get(language)
        if not item:
            failures.append(f"missing {language} package in release manifest")
            continue
        if item.get("registry") != expected["registry"]:
            failures.append(f"{language} registry must be {expected['registry']}")
        if item.get("publish_command") != expected["publish"]:
            failures.append(f"{language} publish_command mismatch")
        if item.get("dry_run_command") != expected["dry_run"]:
            failures.append(f"{language} dry_run_command mismatch")
    return manifest


def check_workflow(repo: Path, failures: list[str]) -> None:
    workflow = read(repo / ".github/workflows/sdk-release.yml")
    for marker in REQUIRED_WORKFLOW_MARKERS:
        if marker not in workflow:
            failures.append(f"sdk-release.yml missing {marker!r}")
    if "if: ${{ github.event_name == 'workflow_dispatch' && inputs.publish" not in workflow:
        failures.append("sdk-release.yml publish job must be workflow_dispatch and publish gated")


def check_docs(repo: Path, failures: list[str]) -> None:
    for relative, markers in REQUIRED_DOC_MARKERS.items():
        text = read(repo / relative)
        for marker in markers:
            if marker not in text:
                failures.append(f"{relative} missing {marker!r}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/sdk-registry-gate/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parent.parent
    failures: list[str] = []
    started_at = utc_now()
    try:
        manifest = check_manifest(repo, failures)
        check_workflow(repo, failures)
        check_docs(repo, failures)
    except Exception as error:  # noqa: BLE001 - release gate reports all failures as JSON.
        manifest = {}
        failures.append(str(error))

    report = {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "started_at": started_at,
        "finished_at": utc_now(),
        "manual_only": True,
        "tag_gated": True,
        "environment": "sdk-release",
        "packages": [
            {"language": language, **expected}
            for language, expected in sorted(REQUIRED_PACKAGES.items())
        ],
        "registry_gate": manifest.get("registry_gate", {}),
        "failures": failures,
    }
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print("SDK REGISTRY GATE CHECK FAILED:")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print(f"sdk registry gate check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
