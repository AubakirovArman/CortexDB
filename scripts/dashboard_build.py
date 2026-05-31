#!/usr/bin/env python3
"""Build or verify the dependency-free dashboard static asset bundle."""

from __future__ import annotations

import argparse
import json
import filecmp
import shutil
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = ROOT / "web" / "dashboard" / "src"
OUT_DIR = ROOT / "crates" / "cortex-server" / "assets" / "dashboard" / "v1"
DIST_DIR = ROOT / "web" / "dashboard" / "dist"
DIST_ASSET_DIR = DIST_DIR / "dashboard" / "assets" / "v1"
ASSETS = ("index.html", "style.css", "app.js", "dashboard_manifest.json")
STATIC_ASSETS = ("style.css", "app.js", "dashboard_manifest.json")
ROUTES = (
    "overview",
    "cells",
    "search",
    "ann-eval",
    "aql",
    "context",
    "verify",
    "ingest",
    "storage",
    "cluster",
)


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
    validate_manifest()


def validate_manifest() -> None:
    manifest_path = SRC_DIR / "dashboard_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    routes = tuple(route.get("id") for route in manifest.get("routes", []))
    if routes != ROUTES:
        raise SystemExit(
            "dashboard_manifest.json routes must match dashboard build routes"
        )
    if manifest.get("stack") != "dependency-free-static-html-css-js":
        raise SystemExit("dashboard_manifest.json must declare the dashboard stack")


def check_assets() -> int:
    assert_source_complete()
    stale = []
    missing = []
    for name in ASSETS:
        source = SRC_DIR / name
        output = OUT_DIR / name
        if not output.is_file():
            missing.append(str(output.relative_to(ROOT)))
        elif not filecmp.cmp(source, output, shallow=False):
            stale.append(str(output.relative_to(ROOT)))
    dist_index = DIST_DIR / "index.html"
    if not dist_index.is_file():
        missing.append(str(dist_index.relative_to(ROOT)))
    elif not filecmp.cmp(SRC_DIR / "index.html", dist_index, shallow=False):
        stale.append(str(dist_index.relative_to(ROOT)))
    dashboard_index = DIST_DIR / "dashboard" / "index.html"
    if not dashboard_index.is_file():
        missing.append(str(dashboard_index.relative_to(ROOT)))
    elif not filecmp.cmp(SRC_DIR / "index.html", dashboard_index, shallow=False):
        stale.append(str(dashboard_index.relative_to(ROOT)))
    for route in ROUTES:
        route_index = DIST_DIR / "dashboard" / route / "index.html"
        if not route_index.is_file():
            missing.append(str(route_index.relative_to(ROOT)))
        elif not filecmp.cmp(SRC_DIR / "index.html", route_index, shallow=False):
            stale.append(str(route_index.relative_to(ROOT)))
    for name in STATIC_ASSETS:
        source = SRC_DIR / name
        output = DIST_ASSET_DIR / name
        if not output.is_file():
            missing.append(str(output.relative_to(ROOT)))
        elif not filecmp.cmp(source, output, shallow=False):
            stale.append(str(output.relative_to(ROOT)))
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
    DIST_ASSET_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(SRC_DIR / "index.html", DIST_DIR / "index.html")
    shutil.copyfile(SRC_DIR / "index.html", DIST_DIR / "dashboard" / "index.html")
    for route in ROUTES:
        route_dir = DIST_DIR / "dashboard" / route
        route_dir.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(SRC_DIR / "index.html", route_dir / "index.html")
    for name in STATIC_ASSETS:
        shutil.copyfile(SRC_DIR / name, DIST_ASSET_DIR / name)
    print(
        "OK: built dashboard assets into "
        f"{OUT_DIR.relative_to(ROOT)} and {DIST_DIR.relative_to(ROOT)}"
    )
    return 0


def main() -> int:
    args = parse_args()
    if args.check:
        return check_assets()
    return build_assets()


if __name__ == "__main__":
    raise SystemExit(main())
