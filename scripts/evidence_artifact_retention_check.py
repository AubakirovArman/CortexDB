#!/usr/bin/env python3
"""Validate CortexDB release evidence retention policy against manifests."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


POLICY_SCHEMA = "cortexdb.evidence_artifact_retention_policy.v1"
SECRET_MARKERS = ("api_key", "secret", "token", ".env", "credential")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def read_string_list(data: dict[str, Any], key: str) -> list[str]:
    value = data.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ValueError(f"{key}: expected non-empty string list")
    return list(value)


def validate_path_list(name: str, paths: list[str]) -> list[str]:
    failures: list[str] = []
    seen: set[str] = set()
    for path in paths:
        if path in seen:
            failures.append(f"{name}: duplicate path {path}")
        seen.add(path)
        if path.startswith("/") or ".." in Path(path).parts:
            failures.append(f"{name}: unsafe path {path}")
        lowered = path.lower()
        if name != "local_only_patterns" and any(marker in lowered for marker in SECRET_MARKERS):
            failures.append(f"{name}: secret-like path must stay local-only: {path}")
    return failures


def artifact_paths(manifest: dict[str, Any]) -> set[str]:
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list):
        raise ValueError("manifest: artifacts must be a list")
    paths: set[str] = set()
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            raise ValueError("manifest: artifact entry must be an object")
        path = artifact.get("path")
        if not isinstance(path, str) or not path:
            raise ValueError("manifest: artifact path must be a non-empty string")
        paths.add(path)
    return paths


def validate_policy(policy: dict[str, Any]) -> tuple[set[str], set[str], set[str], list[str]]:
    failures: list[str] = []
    if policy.get("schema_version") != POLICY_SCHEMA:
        failures.append(f"schema_version must be {POLICY_SCHEMA}")
    github_assets = read_string_list(policy, "github_release_assets")
    bundle_paths = read_string_list(policy, "release_evidence_bundle")
    release_manifest_paths = read_string_list(policy, "release_manifest_artifacts")
    local_patterns = read_string_list(policy, "local_only_patterns")

    failures.extend(validate_path_list("github_release_assets", github_assets))
    failures.extend(validate_path_list("release_evidence_bundle", bundle_paths))
    failures.extend(validate_path_list("release_manifest_artifacts", release_manifest_paths))
    failures.extend(validate_path_list("local_only_patterns", local_patterns))

    bundle_set = set(bundle_paths)
    github_set = set(github_assets)
    release_manifest_set = set(release_manifest_paths)
    overlap = bundle_set & github_set
    allowed_overlap = {
        "target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz",
        "target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz.sha256",
        "target/dashboard/dashboard-v1.tar.gz",
    }
    unexpected = sorted(overlap - allowed_overlap)
    if unexpected:
        failures.append(f"unexpected GitHub/bundle overlap: {unexpected}")
    return github_set, bundle_set, release_manifest_set, failures


def run(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    policy = read_json(repo / args.policy)
    github_assets, bundle_paths, release_manifest_paths, failures = validate_policy(policy)

    if args.bundle_manifest:
        bundle_manifest = read_json(repo / args.bundle_manifest)
        missing = sorted(artifact_paths(bundle_manifest) - bundle_paths)
        if missing:
            failures.append(f"bundle manifest has unclassified artifacts: {missing}")

    if args.release_manifest:
        release_manifest = read_json(repo / args.release_manifest)
        allowed = github_assets | bundle_paths | release_manifest_paths
        missing = sorted(artifact_paths(release_manifest) - allowed)
        if missing:
            failures.append(f"release manifest has unclassified artifacts: {missing}")

    return {
        "schema_version": "cortexdb.evidence_artifact_retention_report.v1",
        "status": "passed" if not failures else "failed",
        "policy": args.policy,
        "github_release_asset_count": len(github_assets),
        "release_evidence_bundle_count": len(bundle_paths),
        "release_manifest_artifact_count": len(release_manifest_paths),
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", default="docs/EVIDENCE_ARTIFACT_RETENTION_POLICY.json")
    parser.add_argument("--bundle-manifest", default="target/release-evidence-bundle/manifest.json")
    parser.add_argument("--release-manifest", default="target/release-artifact-manifest/manifest.json")
    parser.add_argument("--report", default="target/evidence-artifact-retention/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = repo_root()
    try:
        report = run(args)
    except Exception as error:  # noqa: BLE001 - release gate must report failures.
        print(f"error: {error}", file=sys.stderr)
        return 1
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"evidence retention report: {report_path}")
    for failure in report["failures"]:
        print(f"failure: {failure}", file=sys.stderr)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
