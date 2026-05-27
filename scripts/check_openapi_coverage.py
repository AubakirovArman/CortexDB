#!/usr/bin/env python3
"""Verify that every HTTP endpoint in router.rs is documented in openapi.yaml."""

import re
import sys
from pathlib import Path

import yaml


def extract_routes_from_router(router_path: Path) -> set[tuple[str, str]]:
    """Parse router.rs for (method, path) tuples."""
    text = router_path.read_text()
    routes = set()
    # Match patterns like ("GET", "/v1/health") or ("POST", "/v1/cell")
    for match in re.finditer(
        r'\("(GET|POST|DELETE|PUT|PATCH)"\s*,\s*"(/[^"]*)"\)',
        text,
    ):
        method, path = match.groups()
        routes.add((method, path))
    # Match wildcard patterns like _ if method == "GET" && path.starts_with("/v1/ingest/jobs/")
    for match in re.finditer(
        r'method\s*==\s*"(GET|POST|DELETE|PUT|PATCH)"\s*&&\s*path\.starts_with\("(/[^"]*)"\)',
        text,
    ):
        method, prefix = match.groups()
        # Convert prefix to OpenAPI path parameter style
        if prefix.endswith("/"):
            path = prefix + "{job_id}"
        else:
            path = prefix + "/{job_id}"
        routes.add((method, path))
    return routes


def extract_paths_from_openapi(openapi_path: Path) -> set[tuple[str, str]]:
    """Parse openapi.yaml for (method, path) tuples."""
    spec = yaml.safe_load(openapi_path.read_text())
    routes = set()
    for path, methods in spec.get("paths", {}).items():
        for method in methods:
            if method == "parameters":
                continue
            routes.add((method.upper(), path))
    return routes


def main() -> int:
    repo = Path(__file__).parent.parent
    router = repo / "crates/cortex-server/src/router.rs"
    openapi = repo / "docs/openapi.yaml"

    router_routes = extract_routes_from_router(router)
    openapi_routes = extract_paths_from_openapi(openapi)

    missing = router_routes - openapi_routes
    extra = openapi_routes - router_routes

    if missing:
        print("ERROR: Routes in router.rs but missing from openapi.yaml:")
        for method, path in sorted(missing):
            print(f"  {method} {path}")
    if extra:
        print("WARNING: Routes in openapi.yaml but not in router.rs:")
        for method, path in sorted(extra):
            print(f"  {method} {path}")

    if not missing and not extra:
        print("OK: router.rs and openapi.yaml are in sync.")
        return 0

    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
