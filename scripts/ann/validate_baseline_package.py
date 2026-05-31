#!/usr/bin/env python3
"""Validate a release-ready ANN baseline package archive."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import tarfile
import tempfile
import unittest
from pathlib import PurePosixPath, Path
from typing import Any

from history_contract import validate_packaged_history
from report_contract import validate_production_report


CORE_REQUIRED_FILES = {
    "baseline_manifest.json",
    "run_manifest.json",
    "report.json",
    "machine_profile.json",
}


def load_json_bytes(data: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"{label}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{label}: expected JSON object")
    return value


def validate_member_path(name: str) -> PurePosixPath:
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or len(path.parts) < 2:
        raise ValueError(f"unsafe archive path: {name}")
    if any(part == "" for part in path.parts):
        raise ValueError(f"invalid archive path: {name}")
    return path


def package_root(paths: list[PurePosixPath]) -> str:
    roots = {path.parts[0] for path in paths}
    if len(roots) != 1:
        raise ValueError("archive must contain exactly one package root")
    return next(iter(roots))


def read_member(tar: tarfile.TarFile, name: str) -> bytes:
    file = tar.extractfile(name)
    if file is None:
        raise ValueError(f"{name}: not a regular readable file")
    return file.read()


def validate_package(
    archive: Path,
    require_production_safe: bool,
    require_history: bool,
    require_ground_truth: bool,
    require_real_embedding_metadata: bool,
) -> dict[str, Any]:
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        for member in members:
            if member.issym() or member.islnk():
                raise ValueError(f"{member.name}: links are not allowed")
        file_members = [member for member in members if member.isfile()]
        paths = [validate_member_path(member.name) for member in file_members]
        root = package_root(paths)
        manifest_name = f"{root}/package_manifest.json"
        names = {member.name for member in file_members}
        if manifest_name not in names:
            raise ValueError("package_manifest.json not found")
        package = load_json_bytes(read_member(tar, manifest_name), manifest_name)
        if package.get("schema_version") != 1:
            raise ValueError("package_manifest.json: unsupported schema_version")
        if package.get("package_id") != root:
            raise ValueError("package_manifest.json: package_id does not match archive root")
        files = package.get("files")
        if not isinstance(files, list):
            raise ValueError("package_manifest.json: files must be a list")
        listed_paths = validate_files(tar, root, files, names)
        required = set(CORE_REQUIRED_FILES)
        if require_history:
            required.add("history.json")
        if require_ground_truth:
            required.add("ground_truth.jsonl")
        if require_real_embedding_metadata:
            required.update({
                "embedding_preflight.json",
                "embedding_export_manifest.json",
            })
        missing = sorted(required.difference(listed_paths))
        if missing:
            raise ValueError(f"package missing required files: {', '.join(missing)}")
        baseline = load_json_bytes(read_member(tar, f"{root}/baseline_manifest.json"), "baseline_manifest.json")
        report = load_json_bytes(read_member(tar, f"{root}/report.json"), "report.json")
        if report.get("passed") is not True:
            raise ValueError("report.json: passed must be true")
        if require_production_safe and report.get("production_safe") is not True:
            raise ValueError("report.json: production_safe must be true")
        if require_production_safe:
            validate_production_report(report)
        if require_history:
            history = load_json_bytes(read_member(tar, f"{root}/history.json"), "history.json")
            source_run_id = str(baseline.get("source_run_id", ""))
            validate_packaged_history(history, source_run_id=source_run_id)
        if require_real_embedding_metadata:
            export = load_json_bytes(
                read_member(tar, f"{root}/embedding_export_manifest.json"),
                "embedding_export_manifest.json",
            )
            if export.get("provider") == "hash-smoke":
                raise ValueError("embedding_export_manifest.json: hash-smoke is not real embedding evidence")
        return {
            "archive": str(archive),
            "package_id": root,
            "baseline_id": package.get("baseline_id", ""),
            "file_count": len(listed_paths),
            "production_safe": bool(report.get("production_safe")),
            "passed": True,
        }


def validate_files(
    tar: tarfile.TarFile,
    root: str,
    files: list[Any],
    names: set[str],
) -> set[str]:
    listed_paths: set[str] = set()
    for item in files:
        if not isinstance(item, dict):
            raise ValueError("package_manifest.json: file entry must be an object")
        raw_path = item.get("path")
        if not isinstance(raw_path, str):
            raise ValueError("package_manifest.json: file path must be a string")
        rel = validate_relative_path(raw_path)
        member_name = f"{root}/{rel}"
        if member_name not in names:
            raise ValueError(f"{raw_path}: listed file not present in archive")
        data = read_member(tar, member_name)
        if item.get("size_bytes") != len(data):
            raise ValueError(f"{raw_path}: size mismatch")
        if item.get("sha256") != hashlib.sha256(data).hexdigest():
            raise ValueError(f"{raw_path}: sha256 mismatch")
        listed_paths.add(rel)
    return listed_paths


def validate_relative_path(value: str) -> str:
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or len(path.parts) == 0:
        raise ValueError(f"unsafe file path in package manifest: {value}")
    return path.as_posix()


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--require-production-safe", action="store_true")
    parser.add_argument("--require-history", action="store_true")
    parser.add_argument("--require-ground-truth", action="store_true")
    parser.add_argument("--require-real-embedding-metadata", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    summary = validate_package(
        args.archive,
        args.require_production_safe,
        args.require_history,
        args.require_ground_truth,
        args.require_real_embedding_metadata,
    )
    print(json.dumps(summary, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def build_archive(
        self,
        root: Path,
        production_safe: bool = True,
        embedding_provider: str = "command",
        omit_report_field: str = "",
        history: dict[str, Any] | None = None,
    ) -> Path:
        from package_baseline import package_baseline

        bundle = root / "baseline"
        bundle.mkdir()
        (bundle / "baseline_manifest.json").write_text(
            '{"baseline_id":"baseline","source_run_id":"smoke"}\n',
            encoding="utf-8",
        )
        (bundle / "run_manifest.json").write_text('{"run_id":"smoke"}\n', encoding="utf-8")
        report = {
            "passed": True,
            "production_safe": production_safe,
            "require_production_safe": True,
            "required_min_recall_q16": 49_151,
            "required_min_mean_recall_q16": 49_151,
            "allowed_p95_latency_nanos": 100,
            "allowed_max_latency_nanos": 200,
            "hnsw_layer_count": 4,
            "upper_layers": 2,
            "upper_graph_edges": 3,
            "min_observed_recall_q16": 65_535,
            "mean_recall_q16": 65_535,
            "p95_latency_nanos": 10,
            "max_latency_nanos": 20,
        }
        if omit_report_field:
            report.pop(omit_report_field)
        (bundle / "report.json").write_text(json.dumps(report) + "\n", encoding="utf-8")
        (bundle / "machine_profile.json").write_text('{"schema_version":1}\n', encoding="utf-8")
        (bundle / "history.json").write_text(json.dumps(history or clean_history()) + "\n", encoding="utf-8")
        (bundle / "ground_truth.jsonl").write_text('{"name":"q","candidates":[1]}\n', encoding="utf-8")
        (bundle / "embedding_preflight.json").write_text('{"embedding_model":"m"}\n', encoding="utf-8")
        (bundle / "embedding_export_manifest.json").write_text(
            json.dumps({"provider": embedding_provider}) + "\n",
            encoding="utf-8",
        )
        archive = root / "baseline.tar.gz"
        package_baseline(bundle, archive, "baseline", "2026-01-01T00:00:00Z")
        return archive

    def test_valid_package_passes_contract(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            archive = self.build_archive(Path(raw_dir))
            summary = validate_package(archive, True, True, True, True)
        self.assertEqual(summary["package_id"], "baseline")
        self.assertTrue(summary["production_safe"])

    def test_production_safe_requirement_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            archive = self.build_archive(Path(raw_dir), production_safe=False)
            with self.assertRaises(ValueError):
                validate_package(archive, True, True, True, True)

    def test_real_embedding_metadata_rejects_hash_smoke(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            archive = self.build_archive(Path(raw_dir), embedding_provider="hash-smoke")
            with self.assertRaises(ValueError):
                validate_package(archive, True, True, True, True)

    def test_production_package_requires_gate_policy(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            archive = self.build_archive(Path(raw_dir), omit_report_field="required_min_recall_q16")
            with self.assertRaises(ValueError):
                validate_package(archive, True, True, True, False)

    def test_history_regression_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            bad_history = clean_history()
            bad_history["regression_count"] = 1
            archive = self.build_archive(Path(raw_dir), history=bad_history)
            with self.assertRaises(ValueError):
                validate_package(archive, True, True, True, False)


def clean_history() -> dict[str, Any]:
    return {
        "run_count": 1,
        "corpus_count": 1,
        "regression_count": 0,
        "runs": [{"run_id": "smoke", "production_safe": True}],
        "corpora": [{"run_count": 1, "latest_run_id": "smoke", "latest_production_safe": True}],
    }


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
