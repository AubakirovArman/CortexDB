#!/usr/bin/env python3
"""Check local Markdown links without external dependencies."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
IGNORED_PREFIXES = (
    "http://",
    "https://",
    "mailto:",
    "#",
    "app://",
    "plugin://",
)


def main() -> int:
    args = parse_args()
    paths = markdown_paths(args.root)
    missing: list[tuple[Path, str]] = []
    for path in paths:
        text = path.read_text(errors="ignore")
        for match in LINK_RE.finditer(text):
            target = match.group(1).strip()
            if should_skip(target):
                continue
            base = target.split("#", 1)[0]
            candidate = (path.parent / base).resolve()
            if not candidate.exists():
                missing.append((path, target))

    if missing:
        for path, target in missing[: args.max_errors]:
            print(f"{path}: missing {target}")
        if len(missing) > args.max_errors:
            print(f"... {len(missing) - args.max_errors} more")
        print(f"missing_count={len(missing)}")
        return 1

    print(f"markdown links ok: {len(paths)} files")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--max-errors", type=int, default=200)
    return parser.parse_args()


def markdown_paths(root: Path) -> list[Path]:
    output = subprocess.check_output(
        [
            "git",
            "-C",
            str(root),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "*.md",
        ],
        text=True,
    )
    paths = [root / line for line in output.splitlines()]
    return sorted(path for path in paths if path.exists())


def should_skip(target: str) -> bool:
    if not target or target.startswith("<"):
        return True
    if target.startswith(IGNORED_PREFIXES):
        return True
    base = target.split("#", 1)[0]
    return not base or base.startswith("http")


if __name__ == "__main__":
    raise SystemExit(main())
