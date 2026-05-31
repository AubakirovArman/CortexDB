#!/usr/bin/env python3
"""Package and validate CortexDB binary release artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import tarfile
import tempfile
import unittest
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path


BINARIES = ("cortexdb", "cortex-server")


@dataclass(frozen=True)
class FileEntry:
    path: str
    size_bytes: int
    sha256: str


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def executable_path(bin_dir: Path, name: str) -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    path = bin_dir / f"{name}{suffix}"
    if not path.is_file():
        raise ValueError(f"missing binary: {path}")
    mode = path.stat().st_mode
    if not mode & stat.S_IXUSR:
        raise ValueError(f"binary is not executable: {path}")
    return path


def install_doc(package_id: str, platform: str) -> str:
    return f"""# CortexDB Binary Install

Package: `{package_id}`
Platform: `{platform}`

This archive contains:

- `bin/cortexdb`
- `bin/cortex-server`
- `SHA256SUMS`
- `package_manifest.json`

Install locally:

```bash
tar -xzf {package_id}.tar.gz
install -m 0755 {package_id}/bin/cortexdb ~/.local/bin/cortexdb
install -m 0755 {package_id}/bin/cortex-server ~/.local/bin/cortex-server
```

Verify checksums before installing:

```bash
cd {package_id}
sha256sum -c SHA256SUMS
```

Core Alpha binaries are for local/single-node operation. Run `cortexdb validate`
against a backup or staging copy before replacing a production-like local
database binary.
"""


def safe_member_name(name: str) -> None:
    path = Path(name)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"unsafe archive member path: {name}")


def package_binaries(package_id: str, platform: str, version: str, bin_dir: Path, archive: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / package_id
        (root / "bin").mkdir(parents=True)
        (root / "install").mkdir()
        entries: list[FileEntry] = []

        for binary in BINARIES:
            src = executable_path(bin_dir, binary)
            dst = root / "bin" / binary
            dst.write_bytes(src.read_bytes())
            dst.chmod(0o755)
            rel = dst.relative_to(root).as_posix()
            entries.append(FileEntry(rel, dst.stat().st_size, sha256_file(dst)))

        install_path = root / "install" / "INSTALL.md"
        install_path.write_text(install_doc(package_id, platform), encoding="utf-8")
        entries.append(
            FileEntry(
                install_path.relative_to(root).as_posix(),
                install_path.stat().st_size,
                sha256_file(install_path),
            )
        )

        checksums = "\n".join(f"{entry.sha256}  {entry.path}" for entry in entries) + "\n"
        checksums_path = root / "SHA256SUMS"
        checksums_path.write_text(checksums, encoding="utf-8")
        entries.append(
            FileEntry("SHA256SUMS", checksums_path.stat().st_size, sha256_file(checksums_path))
        )

        manifest = {
            "schema_version": 1,
            "package_id": package_id,
            "version": version,
            "platform": platform,
            "created_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "binaries": list(BINARIES),
            "files": [entry.__dict__ for entry in entries],
        }
        manifest_path = root / "package_manifest.json"
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

        archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(root, arcname=package_id)

    digest = sha256_file(archive)
    archive.with_suffix(archive.suffix + ".sha256").write_text(
        f"{digest}  {archive.name}\n", encoding="utf-8"
    )
    print(json.dumps({"archive": str(archive), "sha256": digest, "package_id": package_id}))


def validate_archive(archive: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp)
        with tarfile.open(archive, "r:gz") as tar:
            for member in tar.getmembers():
                safe_member_name(member.name)
            tar.extractall(out)
        roots = [path for path in out.iterdir() if path.is_dir()]
        if len(roots) != 1:
            raise ValueError("archive must contain exactly one root directory")
        root = roots[0]
        manifest = json.loads((root / "package_manifest.json").read_text(encoding="utf-8"))
        if manifest.get("schema_version") != 1:
            raise ValueError("unsupported binary package manifest schema")
        for binary in BINARIES:
            path = root / "bin" / binary
            if not path.is_file():
                raise ValueError(f"missing packaged binary: {binary}")
            if not path.stat().st_mode & stat.S_IXUSR:
                raise ValueError(f"packaged binary is not executable: {binary}")
        for item in manifest.get("files", []):
            path = root / item["path"]
            if path.stat().st_size != item["size_bytes"]:
                raise ValueError(f"{item['path']}: size mismatch")
            if sha256_file(path) != item["sha256"]:
                raise ValueError(f"{item['path']}: sha256 mismatch")
    print(json.dumps({"archive": str(archive), "passed": True}))


class PackageBinaryTests(unittest.TestCase):
    def test_package_and_validate_fake_binaries(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            for binary in BINARIES:
                path = bin_dir / binary
                path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                path.chmod(0o755)
            archive = root / "cortexdb-test.tar.gz"
            package_binaries("cortexdb-test", "test-platform", "test", bin_dir, archive)
            validate_archive(archive)
            self.assertTrue(archive.with_suffix(archive.suffix + ".sha256").is_file())


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-id", default="cortexdb-local")
    parser.add_argument("--platform", default=f"{os.uname().sysname.lower()}-{os.uname().machine}")
    parser.add_argument("--version", default="dev")
    parser.add_argument("--bin-dir", default="target/release")
    parser.add_argument("--archive", default="target/release-artifacts/cortexdb-local.tar.gz")
    parser.add_argument("--validate", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(PackageBinaryTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    archive = Path(args.archive)
    if args.validate:
        validate_archive(archive)
    else:
        package_binaries(args.package_id, args.platform, args.version, Path(args.bin_dir), archive)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
