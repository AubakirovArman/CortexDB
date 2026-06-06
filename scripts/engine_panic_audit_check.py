#!/usr/bin/env python3
"""Fail if production core paths contain direct panic helpers."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOTS = (
    "crates/cortex-core/src",
    "crates/cortex-engine/src",
    "crates/cortex-storage/src",
)
PATTERNS = (".unwrap()", ".expect(", "panic!")
SKIP_PATH_PARTS = ("/src/bin/", "/tests/")
SKIP_FILE_SUFFIXES = ("tests.rs", "_tests.rs")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/engine-panic-audit/report.json")
    return parser.parse_args()


def is_skipped_path(path: Path) -> bool:
    relative = path.relative_to(ROOT).as_posix()
    return any(part in relative for part in SKIP_PATH_PARTS) or relative.endswith(
        SKIP_FILE_SUFFIXES
    )


def line_is_doc(line: str) -> bool:
    stripped = line.lstrip()
    return stripped.startswith("///") or stripped.startswith("//!")


def update_test_module_state(
    line: str,
    cfg_test_pending: bool,
    test_depth: int | None,
) -> tuple[bool, int | None]:
    stripped = line.strip()
    if test_depth is not None:
        test_depth += line.count("{") - line.count("}")
        if test_depth <= 0:
            return False, None
        return False, test_depth
    if stripped == "#[cfg(test)]":
        return True, None
    if cfg_test_pending and stripped.startswith("mod tests"):
        depth = line.count("{") - line.count("}")
        return False, max(depth, 1)
    if stripped and not stripped.startswith("#"):
        return False, None
    return cfg_test_pending, None


def audit_file(path: Path) -> list[dict[str, object]]:
    findings: list[dict[str, object]] = []
    cfg_test_pending = False
    test_depth: int | None = None
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        cfg_test_pending, test_depth = update_test_module_state(
            line, cfg_test_pending, test_depth
        )
        if test_depth is not None or line_is_doc(line):
            continue
        for pattern in PATTERNS:
            if pattern in line:
                findings.append(
                    {
                        "path": path.relative_to(ROOT).as_posix(),
                        "line": line_no,
                        "pattern": pattern,
                        "text": line.strip(),
                    }
                )
    return findings


def main() -> int:
    args = parse_args()
    findings: list[dict[str, object]] = []
    for root in SOURCE_ROOTS:
        for path in sorted((ROOT / root).rglob("*.rs")):
            if is_skipped_path(path):
                continue
            findings.extend(audit_file(path))

    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": "cortexdb.engine_panic_audit.v1",
        "ok": not findings,
        "source_roots": SOURCE_ROOTS,
        "patterns": PATTERNS,
        "findings": findings,
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    if findings:
        for finding in findings:
            print(
                "ERROR: {path}:{line}: {pattern}: {text}".format(**finding)
            )
        return 1
    print(f"engine panic audit passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
