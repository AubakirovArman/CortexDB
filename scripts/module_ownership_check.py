#!/usr/bin/env python3
"""Validate CortexDB module ownership documentation."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


REQUIRED_OWNER_AREAS = [
    "storage",
    "search",
    "context",
    "verify",
    "ingestion",
    "server",
    "cli",
    "sdk",
]

REQUIRED_CRATES = [
    "cortex-aql",
    "cortex-core",
    "cortex-storage",
    "cortex-engine",
    "cortex-cli",
    "cortex-server",
    "cortex-sdk",
]

REQUIRED_PUBLIC_FACADES = [
    "Database",
    "DatabaseOptions",
    "EngineConfig",
    "EngineFeatureFlags",
    "EngineError",
    "EngineErrorCode",
    "EngineResult",
    "DbOperation",
    "ContextPack",
    "StorageStats",
    "StorageValidationReport",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--doc", default="docs/MODULE_OWNERSHIP.md")
    parser.add_argument("--lib", default="crates/cortex-engine/src/lib.rs")
    parser.add_argument("--report", default="target/module-ownership/report.json")
    return parser.parse_args()


def engine_modules(lib_text: str) -> list[str]:
    modules: set[str] = set()
    for line in lib_text.splitlines():
        match = re.match(r"^(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;", line)
        if match:
            modules.add(match.group(1))
    return sorted(modules)


def require_contains(errors: list[str], text: str, token: str, label: str) -> None:
    if token not in text:
        errors.append(f"missing {label}: {token}")


def main() -> int:
    args = parse_args()
    doc_path = Path(args.doc)
    lib_path = Path(args.lib)
    report_path = Path(args.report)
    errors: list[str] = []

    if not doc_path.exists():
        errors.append(f"missing ownership doc: {doc_path}")
        doc_text = ""
    else:
        doc_text = doc_path.read_text(encoding="utf-8")

    if not lib_path.exists():
        errors.append(f"missing engine lib: {lib_path}")
        lib_text = ""
    else:
        lib_text = lib_path.read_text(encoding="utf-8")

    for area in REQUIRED_OWNER_AREAS:
        require_contains(errors, doc_text, f"| {area} |", "owner area")

    for crate in REQUIRED_CRATES:
        require_contains(errors, doc_text, f"`{crate}`", "crate boundary")

    for facade in REQUIRED_PUBLIC_FACADES:
        require_contains(errors, doc_text, facade, "stable facade")

    modules = engine_modules(lib_text)
    for module in modules:
        require_contains(errors, doc_text, f"`{module}`", "engine module ownership")

    if "Status: Epic 30 module ownership boundary map." not in doc_text:
        errors.append("MODULE_OWNERSHIP.md status does not identify Epic 30")

    if "make module-ownership-check" not in doc_text:
        errors.append("MODULE_OWNERSHIP.md does not document the ownership gate")

    report = {
        "ok": not errors,
        "doc": str(doc_path),
        "lib": str(lib_path),
        "owner_areas_checked": REQUIRED_OWNER_AREAS,
        "crates_checked": REQUIRED_CRATES,
        "facades_checked": REQUIRED_PUBLIC_FACADES,
        "engine_modules_checked": modules,
        "errors": errors,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if errors:
        for error in errors:
            print(f"module ownership check failed: {error}")
        return 1
    print(f"module ownership check passed: {report_path.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
