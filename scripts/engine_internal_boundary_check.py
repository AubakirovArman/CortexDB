#!/usr/bin/env python3
"""Check that server and SDK code use the cortex_engine crate-root facade."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


DEFAULT_PATHS = [
    "crates/cortex-server",
    "crates/cortex-sdk",
    "sdk",
]

SKIP_DIRS = {
    ".git",
    ".pytest_cache",
    "__pycache__",
    "target",
    "node_modules",
    "dist",
}

TEXT_EXTENSIONS = {
    ".rs",
    ".py",
    ".ts",
    ".js",
    ".mjs",
    ".cjs",
    ".d.ts",
    ".md",
    ".json",
    ".toml",
    ".yaml",
    ".yml",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lib", default="crates/cortex-engine/src/lib.rs")
    parser.add_argument("--doc", default="docs/ENGINE_INTERNAL_BOUNDARIES.md")
    parser.add_argument("--report", default="target/engine-internal-boundary/report.json")
    parser.add_argument("--path", action="append", dest="paths", default=[])
    return parser.parse_args()


def engine_modules(lib_text: str) -> list[str]:
    modules: set[str] = set()
    for line in lib_text.splitlines():
        match = re.match(r"^(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;", line)
        if match:
            modules.add(match.group(1))
    return sorted(modules)


def iter_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    if root.is_file():
        return [root]
    files: list[Path] = []
    for path in root.rglob("*"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.is_file() and path.suffix in TEXT_EXTENSIONS:
            files.append(path)
    return sorted(files)


def scan_file(path: Path, modules: list[str]) -> list[dict[str, object]]:
    text = path.read_text(encoding="utf-8", errors="replace")
    violations: list[dict[str, object]] = []
    for line_no, line in enumerate(text.splitlines(), start=1):
        for module in modules:
            token = f"cortex_engine::{module}::"
            if token in line:
                violations.append(
                    {
                        "path": str(path),
                        "line": line_no,
                        "module": module,
                        "token": token,
                        "text": line.strip(),
                    }
                )
    return violations


def main() -> int:
    args = parse_args()
    lib_path = Path(args.lib)
    doc_path = Path(args.doc)
    report_path = Path(args.report)
    errors: list[str] = []

    if not lib_path.exists():
        errors.append(f"missing engine lib: {lib_path}")
        modules: list[str] = []
    else:
        modules = engine_modules(lib_path.read_text(encoding="utf-8"))

    if not doc_path.exists():
        errors.append(f"missing internal boundary doc: {doc_path}")
    else:
        doc = doc_path.read_text(encoding="utf-8")
        for token in [
            "Status: Epic 31 internal boundary audit.",
            "make engine-internal-boundary-check",
            "cortex_engine::<module>::",
            "crates/cortex-server",
            "crates/cortex-sdk",
            "sdk",
        ]:
            if token not in doc:
                errors.append(f"boundary doc missing token: {token}")

    scan_roots = [Path(p) for p in (args.paths or DEFAULT_PATHS)]
    violations: list[dict[str, object]] = []
    files_checked = 0
    for root in scan_roots:
        files = iter_files(root)
        files_checked += len(files)
        for path in files:
            violations.extend(scan_file(path, modules))

    if violations:
        errors.extend(
            f"{v['path']}:{v['line']}: forbidden engine internal module path {v['token']}"
            for v in violations
        )

    report = {
        "ok": not errors,
        "modules_checked": modules,
        "paths_checked": [str(path) for path in scan_roots],
        "files_checked": files_checked,
        "violations": violations,
        "errors": errors,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if errors:
        for error in errors:
            print(f"engine internal boundary check failed: {error}")
        return 1
    print(f"engine internal boundary check passed: {report_path.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
