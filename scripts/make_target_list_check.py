#!/usr/bin/env python3
"""Write or compare the public make target list."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


TARGET_RE = re.compile(r"^[a-z][a-zA-Z0-9_-]*:")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="Repository root.")
    parser.add_argument("--output", required=True, help="Target list output path.")
    parser.add_argument("--expected", help="Optional target list to compare against.")
    return parser.parse_args()


def make_database(root: Path) -> str:
    result = subprocess.run(
        ["make", "-C", str(root), "-qp"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode not in (0, 1):
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    return result.stdout


def target_list(root: Path) -> list[str]:
    targets = {
        line.strip()
        for line in make_database(root).splitlines()
        if TARGET_RE.match(line)
    }
    return sorted(targets)


def write_lines(path: Path, lines: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")


def read_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8").splitlines()


def main() -> int:
    args = parse_args()
    targets = target_list(Path(args.root).resolve())
    output = Path(args.output)
    write_lines(output, targets)
    if not args.expected:
        print(f"make-target-list: wrote={output} targets={len(targets)}")
        return 0

    expected = read_lines(Path(args.expected))
    if targets == expected:
        print(f"make-target-list: status=ok targets={len(targets)}")
        return 0

    print(
        "make-target-list: status=failed "
        f"expected={len(expected)} actual={len(targets)}"
    )
    expected_set = set(expected)
    actual_set = set(targets)
    for target in sorted(expected_set - actual_set)[:20]:
        print(f"missing: {target}")
    for target in sorted(actual_set - expected_set)[:20]:
        print(f"added: {target}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
