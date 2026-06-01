#!/usr/bin/env python3
"""Package SDK examples as repeatable release artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import tarfile
from pathlib import Path
from typing import Any


EXAMPLE_FILES = [
    "sdk/README.md",
    "sdk/release-manifest.json",
    "sdk/python/README.md",
    "sdk/python/examples/basic.py",
    "sdk/typescript/README.md",
    "sdk/typescript/examples/basic.mjs",
    "crates/cortex-sdk/examples/basic.rs",
    "crates/cortex-sdk/examples/live_contract.rs",
    "docs/SDK_QUICKSTART.md",
    "docs/SDK_RELEASE.md",
    "docs/SDK_DEPRECATION_POLICY.md",
]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def workspace_version(repo: Path) -> str:
    match = re.search(r'(?m)^version = "([^"]+)"', read_text(repo / "Cargo.toml"))
    if not match:
        raise ValueError("Cargo.toml: workspace version not found")
    return match.group(1)


def load_manifest(repo: Path) -> dict[str, Any]:
    value = json.loads(read_text(repo / "sdk/release-manifest.json"))
    if not isinstance(value, dict):
        raise ValueError("sdk/release-manifest.json: expected object")
    return value


def validate_manifest(manifest: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    artifacts = manifest.get("release_artifacts")
    if not isinstance(artifacts, dict):
        return ["sdk/release-manifest.json: release_artifacts missing"]
    if artifacts.get("sdk_examples_archive") != "target/sdk-release-artifacts/cortexdb-sdk-examples-${version}.tar.gz":
        failures.append("sdk/release-manifest.json: sdk_examples_archive mismatch")
    examples = artifacts.get("sdk_examples")
    if not isinstance(examples, list):
        failures.append("sdk/release-manifest.json: sdk_examples must be a list")
        return failures
    missing = sorted(set(EXAMPLE_FILES) - set(examples))
    extra = sorted(set(examples) - set(EXAMPLE_FILES))
    if missing:
        failures.append(f"sdk/release-manifest.json: missing examples {missing}")
    if extra:
        failures.append(f"sdk/release-manifest.json: unknown examples {extra}")
    return failures


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def package_examples(repo: Path, archive: Path) -> None:
    archive.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "w:gz") as tar:
        for relative in EXAMPLE_FILES:
            source = repo / relative
            if not source.exists():
                raise FileNotFoundError(relative)
            tar.add(source, arcname=relative)


def validate_archive(archive: Path) -> list[str]:
    failures: list[str] = []
    with tarfile.open(archive, "r:gz") as tar:
        names = set(tar.getnames())
    for relative in EXAMPLE_FILES:
        if relative not in names:
            failures.append(f"archive missing {relative}")
    return failures


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/sdk-release-artifacts")
    parser.add_argument("--report", default="target/sdk-release-artifacts/report.json")
    parser.add_argument("--archive")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = Path(__file__).resolve().parent.parent
    output_root = repo / args.root
    report_path = repo / args.report
    failures: list[str] = []

    try:
        version = workspace_version(repo)
        manifest = load_manifest(repo)
        failures.extend(validate_manifest(manifest))
        archive = Path(args.archive) if args.archive else output_root / f"cortexdb-sdk-examples-{version}.tar.gz"
        if not archive.is_absolute():
            archive = repo / archive
        if not failures:
            package_examples(repo, archive)
            failures.extend(validate_archive(archive))
        checksum = sha256_file(archive) if archive.exists() else None
        if checksum:
            archive.with_suffix(archive.suffix + ".sha256").write_text(f"{checksum}  {archive.name}\n", encoding="utf-8")
        report = {
            "schema_version": 1,
            "status": "passed" if not failures else "failed",
            "workspace_version": version,
            "artifact": str(archive.relative_to(repo)) if archive.exists() else None,
            "sha256": checksum,
            "examples": EXAMPLE_FILES,
            "failures": failures,
        }
    except Exception as error:  # noqa: BLE001 - release gate reports all failures as JSON.
        report = {
            "schema_version": 1,
            "status": "failed",
            "artifact": None,
            "sha256": None,
            "examples": EXAMPLE_FILES,
            "failures": [str(error)],
        }

    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"sdk release artifacts check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
