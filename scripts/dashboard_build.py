#!/usr/bin/env python3
"""Build or verify the dependency-free dashboard static asset bundle."""

from __future__ import annotations

import argparse
import filecmp
import shutil
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = ROOT / "web" / "dashboard" / "src"
OUT_DIR = ROOT / "crates" / "cortex-server" / "assets" / "dashboard" / "v1"
ASSETS = ("index.html", "style.css", "app.js")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify output assets match source without writing files",
    )
    return parser.parse_args()


def assert_source_complete() -> None:
    missing = [name for name in ASSETS if not (SRC_DIR / name).is_file()]
    if missing:
        joined = ", ".join(missing)
        raise SystemExit(f"missing dashboard source assets: {joined}")


def check_assets() -> int:
    assert_source_complete()
    stale = []
    missing = []
    for name in ASSETS:
        source = SRC_DIR / name
        output = OUT_DIR / name
        if not output.is_file():
            missing.append(name)
        elif not filecmp.cmp(source, output, shallow=False):
            stale.append(name)
    if missing or stale:
        if missing:
            print("missing built dashboard assets: " + ", ".join(missing), file=sys.stderr)
        if stale:
            print("stale built dashboard assets: " + ", ".join(stale), file=sys.stderr)
        print("run: make dashboard-build", file=sys.stderr)
        return 1
    print("OK: dashboard static assets are in sync")
    return 0


def build_assets() -> int:
    assert_source_complete()
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name in ASSETS:
        shutil.copyfile(SRC_DIR / name, OUT_DIR / name)
    print(f"OK: built dashboard assets into {OUT_DIR.relative_to(ROOT)}")
    return 0


def main() -> int:
    args = parse_args()
    if args.check:
        return check_assets()
    return build_assets()


if __name__ == "__main__":
    raise SystemExit(main())
