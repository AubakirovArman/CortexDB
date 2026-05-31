#!/usr/bin/env python3
"""Attach real-embedding provenance files to an ANN corpus run."""

from __future__ import annotations

import argparse
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def copy_metadata(src: Path | None, run_dir: Path, dst_name: str) -> str:
    if src is None:
        return ""
    if not src.is_file():
        raise ValueError(f"{src}: metadata file not found")
    dst = run_dir / dst_name
    shutil.copy2(src, dst)
    return str(dst)


def attach_metadata(
    run_dir: Path,
    preflight: Path,
    export_manifest: Path,
    source_archive_manifest: Path | None,
) -> dict[str, Any]:
    manifest_path = run_dir / "manifest.json"
    if not manifest_path.is_file():
        raise ValueError(f"{manifest_path}: run manifest not found")
    manifest = load_json(manifest_path)
    preflight_json = load_json(preflight)
    export_json = load_json(export_manifest)
    if export_json.get("provider") == "hash-smoke":
        raise ValueError("real embedding metadata cannot use hash-smoke provider")
    manifest["embedding_preflight"] = copy_metadata(
        preflight, run_dir, "embedding_preflight.json"
    )
    manifest["embedding_export_manifest"] = copy_metadata(
        export_manifest, run_dir, "embedding_export_manifest.json"
    )
    source_archive_path = copy_metadata(
        source_archive_manifest, run_dir, "source_archive_manifest.json"
    )
    if source_archive_path:
        manifest["source_archive_manifest"] = source_archive_path
    manifest["embedding_model"] = preflight_json.get("embedding_model", "")
    manifest["embedding_endpoint_origin"] = preflight_json.get(
        "embedding_endpoint_origin", ""
    )
    manifest["embedding_provider"] = export_json.get("provider", "")
    manifest["embedding_dimension"] = export_json.get("dimension", 0)
    manifest_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return manifest


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--preflight", type=Path, required=True)
    parser.add_argument("--export-manifest", type=Path, required=True)
    parser.add_argument("--source-archive-manifest", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    manifest = attach_metadata(
        args.run_dir,
        args.preflight,
        args.export_manifest,
        args.source_archive_manifest,
    )
    print(json.dumps(manifest, ensure_ascii=False, separators=(",", ":")))
    return 0


class SelfTests(unittest.TestCase):
    def test_attach_metadata_updates_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            run_dir = root / "run"
            run_dir.mkdir()
            (run_dir / "manifest.json").write_text(
                '{"run_id":"real"}\n', encoding="utf-8"
            )
            preflight = root / "preflight.json"
            preflight.write_text(
                '{"embedding_model":"model-a","embedding_endpoint_origin":"https://e"}\n',
                encoding="utf-8",
            )
            export = root / "export.json"
            export.write_text(
                '{"provider":"command","dimension":3}\n', encoding="utf-8"
            )
            source = root / "source.json"
            source.write_text('{"sha256":"abc"}\n', encoding="utf-8")
            manifest = attach_metadata(run_dir, preflight, export, source)
            self.assertEqual(manifest["embedding_model"], "model-a")
            self.assertEqual(manifest["embedding_provider"], "command")
            self.assertTrue((run_dir / "embedding_preflight.json").is_file())
            self.assertTrue((run_dir / "embedding_export_manifest.json").is_file())
            self.assertTrue((run_dir / "source_archive_manifest.json").is_file())

    def test_hash_smoke_provider_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            run_dir = root / "run"
            run_dir.mkdir()
            (run_dir / "manifest.json").write_text("{}", encoding="utf-8")
            preflight = root / "preflight.json"
            preflight.write_text("{}", encoding="utf-8")
            export = root / "export.json"
            export.write_text('{"provider":"hash-smoke"}\n', encoding="utf-8")
            with self.assertRaises(ValueError):
                attach_metadata(run_dir, preflight, export, None)


if __name__ == "__main__":
    try:
        raise SystemExit(main(__import__("sys").argv[1:]))
    except ValueError as error:
        print(f"error: {error}", file=__import__("sys").stderr)
        raise SystemExit(2)
