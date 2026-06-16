#!/usr/bin/env python3
"""Validate relative Markdown links in README.md and docs/."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path
from urllib.parse import unquote


INLINE_LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)\n]+)\)")
REFERENCE_LINK_RE = re.compile(r"^\s*\[[^\]]+\]:\s*(\S+)", re.MULTILINE)
FENCED_BLOCK_RE = re.compile(r"```.*?```", re.DOTALL)
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
SKIPPED_PREFIXES = ("#", "<", "/")


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    paths = markdown_paths(root)
    missing: list[tuple[Path, str]] = []

    for path in paths:
        text = strip_fenced_blocks(path.read_text(encoding="utf-8", errors="ignore"))
        for raw_target in iter_markdown_targets(text):
            target = normalize_target(raw_target)
            if should_skip(target):
                continue
            candidate = resolve_target(path, root, target)
            if not candidate.exists():
                missing.append((path, target))

    if missing:
        for path, target in missing[: args.max_errors]:
            print(f"{path.relative_to(root)}: missing {target}")
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
    tracked = subprocess.check_output(
        [
            "git",
            "-C",
            str(root),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "README.md",
            "docs/*.md",
            "docs/**/*.md",
        ],
        text=True,
    )
    paths = [root / line for line in tracked.splitlines()]
    return sorted(path for path in paths if path.exists())


def strip_fenced_blocks(text: str) -> str:
    return FENCED_BLOCK_RE.sub("", text)


def iter_markdown_targets(text: str) -> list[str]:
    inline = [match.group(1) for match in INLINE_LINK_RE.finditer(text)]
    references = [match.group(1) for match in REFERENCE_LINK_RE.finditer(text)]
    return inline + references


def normalize_target(raw_target: str) -> str:
    target = raw_target.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1].strip()
    if " " in target and not target.startswith("<"):
        target = target.split(None, 1)[0]
    return unquote(target)


def should_skip(target: str) -> bool:
    if not target:
        return True
    if target.startswith(SKIPPED_PREFIXES):
        return True
    return bool(SCHEME_RE.match(target))


def resolve_target(source: Path, root: Path, target: str) -> Path:
    base = target.split("#", 1)[0]
    return (source.parent / base).resolve() if base else root


if __name__ == "__main__":
    raise SystemExit(main())
