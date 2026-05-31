#!/usr/bin/env python3
"""Package and validate the standalone dashboard release artifact."""

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
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DIST_DIR = ROOT / "web" / "dashboard" / "dist"
ROUTES = (
    "overview",
    "cells",
    "search",
    "ann-eval",
    "aql",
    "context",
    "verify",
    "ingest",
    "storage",
    "cluster",
)
REQUIRED_FILES = (
    "index.html",
    "dashboard/index.html",
    "dashboard/assets/v1/app.js",
    "dashboard/assets/v1/dashboard_manifest.json",
    "dashboard/assets/v1/reporting_common.js",
    "dashboard/assets/v1/reporting_retrieval.js",
    "dashboard/assets/v1/reporting_operations.js",
    "dashboard/assets/v1/reporting.js",
    "dashboard/assets/v1/style.css",
    *(f"dashboard/{route}/index.html" for route in ROUTES),
)


def validate_package_id(value: str) -> str:
    if not value or any(char in value for char in "/\\\0"):
        raise ValueError("package id must be a single path segment")
    if value in {".", ".."}:
        raise ValueError("package id must not be '.' or '..'")
    return value


def dist_files(dist_dir: Path) -> list[Path]:
    missing = [name for name in REQUIRED_FILES if not (dist_dir / name).is_file()]
    if missing:
        raise ValueError(f"{dist_dir}: missing required dashboard files: {', '.join(missing)}")
    files: list[Path] = []
    for path in sorted(dist_dir.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"{path}: symlinks are not allowed in dashboard packages")
        if path.is_file():
            files.append(path)
    return files


def file_entry(dist_dir: Path, path: Path) -> dict[str, Any]:
    data = path.read_bytes()
    return {
        "path": path.relative_to(dist_dir).as_posix(),
        "size_bytes": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def package_manifest(dist_dir: Path, package_id: str, created_at: str, files: list[Path]) -> dict[str, Any]:
    dashboard_manifest = read_dashboard_manifest(dist_dir)
    return {
        "schema_version": 1,
        "package_id": package_id,
        "created_at": created_at,
        "source_dist": str(dist_dir),
        "asset_root": "/dashboard/assets/v1",
        "frontend_stack": dashboard_manifest["stack"],
        "frontend_manifest": "dashboard/assets/v1/dashboard_manifest.json",
        "entrypoints": ["index.html", "dashboard/index.html", *[f"dashboard/{route}/index.html" for route in ROUTES]],
        "files": [file_entry(dist_dir, path) for path in files],
    }


def read_dashboard_manifest(dist_dir: Path) -> dict[str, Any]:
    path = dist_dir / "dashboard" / "assets" / "v1" / "dashboard_manifest.json"
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("dashboard_manifest.json: unsupported schema_version")
    if manifest.get("stack") != "dependency-free-static-html-css-js":
        raise ValueError("dashboard_manifest.json: unexpected stack")
    if manifest.get("session_policy", {}).get("token_persistence") != "memory-only":
        raise ValueError("dashboard_manifest.json: token persistence must be memory-only")
    routes = tuple(route.get("id") for route in manifest.get("routes", []))
    if routes != ROUTES:
        raise ValueError("dashboard_manifest.json: route list does not match release routes")
    return manifest


def add_bytes(tar: tarfile.TarFile, arcname: str, data: bytes) -> None:
    info = tarfile.TarInfo(arcname)
    info.size = len(data)
    info.mtime = 0
    tar.addfile(info, io.BytesIO(data))


def package_dashboard(dist_dir: Path, archive: Path, package_id: str, created_at: str) -> dict[str, Any]:
    package_id = validate_package_id(package_id)
    dist_dir = dist_dir.resolve()
    files = dist_files(dist_dir)
    manifest = package_manifest(dist_dir, package_id, created_at, files)
    archive.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "w:gz") as tar:
        manifest_bytes = json.dumps(manifest, indent=2, sort_keys=True).encode("utf-8") + b"\n"
        add_bytes(tar, f"{package_id}/package_manifest.json", manifest_bytes)
        for path in files:
            rel = path.relative_to(dist_dir).as_posix()
            add_bytes(tar, f"{package_id}/{rel}", path.read_bytes())
    return {**manifest, "archive": str(archive)}


def validate_member_path(name: str) -> PurePosixPath:
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or len(path.parts) < 2:
        raise ValueError(f"unsafe archive path: {name}")
    return path


def read_member(tar: tarfile.TarFile, name: str) -> bytes:
    file = tar.extractfile(name)
    if file is None:
        raise ValueError(f"{name}: not a readable file")
    return file.read()


def validate_relative_path(value: str) -> str:
    if not isinstance(value, str):
        raise ValueError("package manifest file path must be a string")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"unsafe package file path: {value}")
    return path.as_posix()


def validate_dashboard_package(archive: Path) -> dict[str, Any]:
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        if any(member.issym() or member.islnk() for member in members):
            raise ValueError("dashboard package must not contain links")
        file_members = [member for member in members if member.isfile()]
        paths = [validate_member_path(member.name) for member in file_members]
        roots = {path.parts[0] for path in paths}
        if len(roots) != 1:
            raise ValueError("dashboard package must contain exactly one package root")
        root = next(iter(roots))
        names = {member.name for member in file_members}
        manifest_name = f"{root}/package_manifest.json"
        if manifest_name not in names:
            raise ValueError("package_manifest.json not found")
        manifest = json.loads(read_member(tar, manifest_name).decode("utf-8"))
        if manifest.get("schema_version") != 1:
            raise ValueError("package_manifest.json: unsupported schema_version")
        if manifest.get("package_id") != root:
            raise ValueError("package_manifest.json: package_id does not match archive root")
        if manifest.get("frontend_stack") != "dependency-free-static-html-css-js":
            raise ValueError("package_manifest.json: unexpected frontend_stack")
        listed = validate_files(tar, root, manifest.get("files"), names)
        missing = sorted(set(REQUIRED_FILES).difference(listed))
        if missing:
            raise ValueError(f"dashboard package missing required files: {', '.join(missing)}")
        validate_packaged_dashboard_manifest(tar, root)
        return {"archive": str(archive), "package_id": root, "file_count": len(listed), "passed": True}


def validate_packaged_dashboard_manifest(tar: tarfile.TarFile, root: str) -> None:
    raw = read_member(tar, f"{root}/dashboard/assets/v1/dashboard_manifest.json")
    manifest = json.loads(raw.decode("utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("dashboard_manifest.json: unsupported schema_version")
    if manifest.get("stack") != "dependency-free-static-html-css-js":
        raise ValueError("dashboard_manifest.json: unexpected stack")
    if manifest.get("session_policy", {}).get("token_persistence") != "memory-only":
        raise ValueError("dashboard_manifest.json: token persistence must be memory-only")
    routes = tuple(route.get("id") for route in manifest.get("routes", []))
    if routes != ROUTES:
        raise ValueError("dashboard_manifest.json: route list does not match release routes")


def validate_files(tar: tarfile.TarFile, root: str, files: Any, names: set[str]) -> set[str]:
    if not isinstance(files, list):
        raise ValueError("package_manifest.json: files must be a list")
    listed: set[str] = set()
    for item in files:
        if not isinstance(item, dict):
            raise ValueError("package_manifest.json: file entries must be objects")
        rel = validate_relative_path(item.get("path", ""))
        member_name = f"{root}/{rel}"
        if member_name not in names:
            raise ValueError(f"{rel}: listed file not present")
        data = read_member(tar, member_name)
        if item.get("size_bytes") != len(data):
            raise ValueError(f"{rel}: size mismatch")
        if item.get("sha256") != hashlib.sha256(data).hexdigest():
            raise ValueError(f"{rel}: sha256 mismatch")
        listed.add(rel)
    return listed


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    parser.add_argument("--archive", type=Path, default=ROOT / "target" / "dashboard" / "dashboard-v1.tar.gz")
    parser.add_argument("--package-id", default="dashboard-v1")
    parser.add_argument("--created-at")
    parser.add_argument("--validate", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    if args.validate:
        summary = validate_dashboard_package(args.archive)
    else:
        created_at = args.created_at or datetime.now(timezone.utc).replace(microsecond=0).isoformat()
        summary = package_dashboard(args.dist_dir, args.archive, args.package_id, created_at)
    print(json.dumps(summary, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def write_dist(self, root: Path) -> Path:
        dist = root / "dist"
        (dist / "dashboard" / "assets" / "v1").mkdir(parents=True)
        (dist / "index.html").write_text("<title>CortexDB Console</title>", encoding="utf-8")
        (dist / "dashboard" / "index.html").write_text("<title>CortexDB Console</title>", encoding="utf-8")
        (dist / "dashboard" / "assets" / "v1" / "app.js").write_text("console.log('ok')", encoding="utf-8")
        (dist / "dashboard" / "assets" / "v1" / "reporting_common.js").write_text(
            "window.CortexDashboardReports={helpers:{}}",
            encoding="utf-8",
        )
        (dist / "dashboard" / "assets" / "v1" / "reporting_retrieval.js").write_text(
            "window.CortexDashboardReports.renderSearchReport=()=>{}",
            encoding="utf-8",
        )
        (dist / "dashboard" / "assets" / "v1" / "reporting_operations.js").write_text(
            "window.CortexDashboardReports.renderCellReport=()=>{}",
            encoding="utf-8",
        )
        (dist / "dashboard" / "assets" / "v1" / "reporting.js").write_text(
            "window.CortexDashboardReports={}",
            encoding="utf-8",
        )
        (dist / "dashboard" / "assets" / "v1" / "dashboard_manifest.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "stack": "dependency-free-static-html-css-js",
                    "session_policy": {"token_persistence": "memory-only"},
                    "routes": [{"id": route} for route in ROUTES],
                }
            ),
            encoding="utf-8",
        )
        (dist / "dashboard" / "assets" / "v1" / "style.css").write_text("body{}", encoding="utf-8")
        for route in ROUTES:
            route_dir = dist / "dashboard" / route
            route_dir.mkdir()
            (route_dir / "index.html").write_text(route, encoding="utf-8")
        return dist

    def test_package_validates(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            archive = root / "dashboard.tar.gz"
            package_dashboard(self.write_dist(root), archive, "dashboard", "2026-01-01T00:00:00Z")
            summary = validate_dashboard_package(archive)
        self.assertEqual(summary["package_id"], "dashboard")
        self.assertTrue(summary["passed"])

    def test_missing_route_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            dist = self.write_dist(Path(raw_dir))
            (dist / "dashboard" / "search" / "index.html").unlink()
            with self.assertRaises(ValueError):
                dist_files(dist)


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2)
