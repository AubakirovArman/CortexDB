#!/usr/bin/env python3
"""Report and ratchet-check source file sizes."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "cortexdb.file_size_report.v1"
CODE_EXTENSIONS = {
    ".rs",
    ".py",
    ".sh",
    ".mk",
    ".js",
    ".jsx",
    ".ts",
    ".tsx",
    ".mjs",
    ".cjs",
}
EXCLUDED_DIRS = {".git", ".venv", "__pycache__", "node_modules", "target", "test-results", "erb-submission"}
EXCLUDED_PARTS = {"dist", "snapshots", "golden", "generated"}
EXCLUDED_FILES = {"Cargo.lock", "package-lock.json"}
LIMITS = {
    "source": {"soft": 300, "warning": 600, "hard": 1000},
    "test": {"soft": 600, "warning": 1200, "hard": 2000},
    "makefile": {"soft": 150, "warning": 600, "hard": 1000},
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="Repository root to scan.")
    parser.add_argument("--output", help="Optional JSON report path.")
    parser.add_argument("--baseline", help="Baseline path for --check.")
    parser.add_argument("--write-baseline", help="Write current scan as baseline.")
    parser.add_argument("--check", action="store_true", help="Run ratchet check.")
    return parser.parse_args()


def skip_path(path: Path) -> bool:
    parts = set(path.parts)
    return bool(parts & EXCLUDED_DIRS or parts & EXCLUDED_PARTS)


def should_scan(path: Path) -> bool:
    if path.name in EXCLUDED_FILES or skip_path(path):
        return False
    if path.name == "Makefile" or path.suffix == ".mk":
        return True
    return path.suffix in CODE_EXTENSIONS and path.suffix != ".snap"


def kind_for(path: Path) -> str:
    if path.name == "Makefile":
        return "makefile"
    if (
        "tests" in path.parts
        or path.name == "tests.rs"
        or path.name.startswith("test_")
        or path.name.endswith("_tests.py")
        or path.name.endswith("_tests.rs")
    ):
        return "test"
    return "source"


def bucket_for(lines: int, limits: dict[str, int]) -> str:
    if lines > limits["hard"]:
        return "over_hard"
    if lines > limits["warning"]:
        return "over_warning"
    if lines > limits["soft"]:
        return "over_soft"
    return "ok"


def count_lines(path: Path) -> int:
    try:
        with path.open("rb") as handle:
            return sum(1 for _ in handle)
    except OSError as error:
        raise SystemExit(f"failed to read {path}: {error}") from error


def git_paths(root: Path) -> list[Path] | None:
    command = [
        "git",
        "-C",
        str(root),
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ]
    try:
        result = subprocess.run(command, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except (OSError, subprocess.CalledProcessError):
        return None
    return sorted(Path(part.decode("utf-8")) for part in result.stdout.split(b"\0") if part)


def walk_paths(root: Path) -> list[Path]:
    paths: list[Path] = []
    for current_root, dirnames, filenames in os.walk(root):
        current_path = Path(current_root)
        dirnames[:] = [
            name for name in dirnames if not skip_path((current_path / name).relative_to(root))
        ]
        paths.extend((current_path / name).relative_to(root) for name in filenames)
    return sorted(paths)


def scan(root: Path) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    for relative in git_paths(root) or walk_paths(root):
        if not should_scan(relative):
            continue
        kind = kind_for(relative)
        limits = LIMITS[kind]
        lines = count_lines(root / relative)
        entries.append(
            {
                "path": relative.as_posix(),
                "lines": lines,
                "kind": kind,
                "soft_limit": limits["soft"],
                "warning_limit": limits["warning"],
                "hard_limit": limits["hard"],
                "bucket": bucket_for(lines, limits),
            }
        )
    return sorted(entries, key=lambda item: item["path"])


def summary(entries: list[dict[str, Any]]) -> dict[str, Any]:
    max_entry = max(entries, key=lambda item: item["lines"], default=None)
    return {
        "total_files": len(entries),
        "over_soft": sum(1 for item in entries if item["bucket"] == "over_soft"),
        "over_warning": sum(1 for item in entries if item["bucket"] == "over_warning"),
        "over_hard": sum(1 for item in entries if item["bucket"] == "over_hard"),
        "max_lines": max_entry["lines"] if max_entry else 0,
        "max_path": max_entry["path"] if max_entry else None,
    }


def baseline(entries: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "files": {
            item["path"]: {
                "lines": item["lines"],
                "kind": item["kind"],
                "soft_limit": item["soft_limit"],
                "warning_limit": item["warning_limit"],
                "hard_limit": item["hard_limit"],
            }
            for item in entries
        },
    }


def read_baseline(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise SystemExit(f"failed to read baseline {path}: {error}") from error
    if data.get("schema_version") != SCHEMA_VERSION or not isinstance(data.get("files"), dict):
        raise SystemExit(f"invalid baseline schema in {path}")
    return data


def ratchet(entries: list[dict[str, Any]], data: dict[str, Any]) -> list[dict[str, Any]]:
    """Threshold-aware ratchet.

    The gate fails only on meaningful size problems, not on every byte of growth:
    - a new file created already over its warning limit;
    - an existing file that grows *while over* its warning limit (oversized files
      must not get worse).

    Files under the warning limit may grow freely. This keeps the architectural
    signal (catch large / worsening files) without failing CI on a +1-line change
    to a small file, which previously forced a baseline refresh every few commits.
    """
    violations = []
    baseline_files = data["files"]
    for item in entries:
        old = baseline_files.get(item["path"])
        over_warning = item["lines"] > item["warning_limit"]
        if old is None and over_warning:
            violations.append(
                {
                    "path": item["path"],
                    "reason": "new_file_over_warning_limit",
                    "lines": item["lines"],
                    "limit": item["warning_limit"],
                }
            )
        elif old is not None and over_warning and item["lines"] > int(old["lines"]):
            violations.append(
                {
                    "path": item["path"],
                    "reason": "baseline_growth_over_warning",
                    "lines": item["lines"],
                    "baseline_lines": int(old["lines"]),
                    "delta": item["lines"] - int(old["lines"]),
                    "limit": item["warning_limit"],
                }
            )
    return violations


def write_json(path: str | None, data: dict[str, Any]) -> None:
    if not path:
        return
    output = Path(path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def print_report(data: dict[str, Any]) -> None:
    stats = data["summary"]
    print(
        "file-size-report: "
        f"status={data['status']} files={stats['total_files']} "
        f"over_soft={stats['over_soft']} over_warning={stats['over_warning']} "
        f"over_hard={stats['over_hard']} max={stats['max_lines']}:{stats['max_path']}"
    )
    for item in data["violations"]:
        if "baseline_lines" in item:
            print(
                f"violation: {item['path']} grew "
                f"{item['baseline_lines']} -> {item['lines']} (+{item['delta']}) "
                f"while over warning limit {item['limit']}"
            )
        else:
            print(f"violation: {item['path']} is a new file with {item['lines']} lines > {item['limit']}")


def main() -> int:
    args = parse_args()
    entries = scan(Path(args.root).resolve())
    if args.write_baseline:
        write_json(args.write_baseline, baseline(entries))

    violations: list[dict[str, Any]] = []
    if args.check:
        if not args.baseline:
            raise SystemExit("--check requires --baseline")
        violations = ratchet(entries, read_baseline(Path(args.baseline)))

    report = {
        "schema_version": SCHEMA_VERSION,
        "status": "failed" if violations else "ok",
        "summary": summary(entries),
        "violations": violations,
        "files": entries,
    }
    write_json(args.output, report)
    print_report(report)
    return 1 if violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
