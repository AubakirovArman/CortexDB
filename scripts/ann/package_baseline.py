#!/usr/bin/env python3
"""Package an ANN baseline bundle as a release-ready tar.gz artifact."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import sys
import tarfile
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REQUIRED_FILES = [
    "baseline_manifest.json",
    "run_manifest.json",
    "report.json",
    "machine_profile.json",
]


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def validate_package_id(value: str) -> str:
    if not value or any(char in value for char in "/\\\0"):
        raise ValueError("package id must be a single path segment")
    if value in {".", ".."}:
        raise ValueError("package id must not be '.' or '..'")
    return value


def bundle_files(bundle: Path) -> list[Path]:
    missing = [name for name in REQUIRED_FILES if not (bundle / name).is_file()]
    if missing:
        raise ValueError(f"{bundle}: missing required files: {', '.join(missing)}")
    files = []
    for path in sorted(bundle.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"{path}: symlinks are not allowed in release packages")
        if path.is_file():
            files.append(path)
    return files


def file_entry(bundle: Path, path: Path) -> dict[str, Any]:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    return {
        "path": path.relative_to(bundle).as_posix(),
        "size_bytes": path.stat().st_size,
        "sha256": digest,
    }


def package_manifest(
    bundle: Path,
    package_id: str,
    created_at: str,
    files: list[Path],
) -> dict[str, Any]:
    baseline = load_json(bundle / "baseline_manifest.json")
    report = load_json(bundle / "report.json")
    return {
        "schema_version": 1,
        "package_id": package_id,
        "created_at": created_at,
        "source_bundle": bundle.name,
        "baseline_id": baseline.get("baseline_id", ""),
        "git_sha": baseline.get("git_sha", ""),
        "summary": baseline.get("summary", {}),
        "report_passed": report.get("passed", False),
        "production_safe": report.get("production_safe", False),
        "files": [file_entry(bundle, path) for path in files],
    }


def add_bytes(tar: tarfile.TarFile, arcname: str, data: bytes) -> None:
    info = tarfile.TarInfo(arcname)
    info.size = len(data)
    info.mtime = 0
    tar.addfile(info, io.BytesIO(data))


def package_baseline(bundle: Path, output: Path, package_id: str, created_at: str) -> dict[str, Any]:
    bundle = bundle.resolve()
    package_id = validate_package_id(package_id)
    files = bundle_files(bundle)
    manifest = package_manifest(bundle, package_id, created_at, files)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w:gz") as tar:
        manifest_bytes = json.dumps(manifest, indent=2, sort_keys=True).encode("utf-8") + b"\n"
        add_bytes(tar, f"{package_id}/package_manifest.json", manifest_bytes)
        for path in files:
            arcname = f"{package_id}/{path.relative_to(bundle).as_posix()}"
            tar.add(path, arcname=arcname, recursive=False)
    return {**manifest, "archive": str(output)}


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--package-id")
    parser.add_argument("--created-at")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    package_id = args.package_id or args.baseline_bundle.name
    output = args.output or args.baseline_bundle.with_suffix(".tar.gz")
    created_at = args.created_at or datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    manifest = package_baseline(args.baseline_bundle, output, package_id, created_at)
    sys.stdout.write(json.dumps(manifest, separators=(",", ":")) + "\n")
    return 0


class SelfTests(unittest.TestCase):
    def write_bundle(self, root: Path) -> Path:
        bundle = root / "baseline"
        bundle.mkdir()
        baseline = {
            "baseline_id": "baseline",
            "git_sha": "abc123",
            "summary": {"passed": True},
        }
        report = {"passed": True, "production_safe": True}
        (bundle / "baseline_manifest.json").write_text(json.dumps(baseline), encoding="utf-8")
        (bundle / "run_manifest.json").write_text('{"run_id":"smoke"}\n', encoding="utf-8")
        (bundle / "report.json").write_text(json.dumps(report), encoding="utf-8")
        (bundle / "machine_profile.json").write_text('{"schema_version":1}\n', encoding="utf-8")
        (bundle / "ground_truth.jsonl").write_text('{"name":"q","candidates":[1]}\n', encoding="utf-8")
        return bundle

    def test_package_contains_manifest_and_checksums(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            bundle = self.write_bundle(root)
            output = root / "baseline.tar.gz"
            manifest = package_baseline(bundle, output, "baseline", "2026-01-01T00:00:00Z")
            self.assertTrue(output.exists())
            self.assertEqual(manifest["baseline_id"], "baseline")
            with tarfile.open(output, "r:gz") as tar:
                names = set(tar.getnames())
                self.assertIn("baseline/package_manifest.json", names)
                self.assertIn("baseline/machine_profile.json", names)
                package = json.load(tar.extractfile("baseline/package_manifest.json"))
            self.assertEqual(package["source_bundle"], "baseline")
            self.assertTrue(any(item["path"] == "report.json" for item in package["files"]))

    def test_missing_required_file_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            bundle = self.write_bundle(Path(raw_dir))
            (bundle / "machine_profile.json").unlink()
            with self.assertRaises(ValueError):
                bundle_files(bundle)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
