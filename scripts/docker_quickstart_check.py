#!/usr/bin/env python3
"""Validate Docker quickstart, GHCR release, fixture seed, and dashboard wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def require_markers(path: str, markers: list[str], failures: list[str]) -> dict[str, object]:
    text = read(path)
    for marker in markers:
        require(marker in text, f"{path} missing {marker!r}", failures)
    return {"path": path, "markers_checked": len(markers)}


def check_release_workflow(failures: list[str]) -> dict[str, object]:
    return require_markers(
        ".github/workflows/release.yml",
        [
            "packages: write",
            "release-container:",
            "ghcr.io/${repo}",
            "docker login ghcr.io",
            "docker build",
            "docker push \"${GHCR_IMAGE}:${RELEASE_TAG}\"",
            "docker push \"${GHCR_IMAGE}:latest\"",
            "docker run --rm --entrypoint cortexdb",
        ],
        failures,
    )


def check_dockerfile(failures: list[str]) -> dict[str, object]:
    return require_markers(
        "Dockerfile",
        [
            "COPY Cargo.toml Cargo.lock ./",
            "COPY crates/ ./crates/",
            "COPY fixtures/ ./fixtures/",
            "cargo build --release -p cortex-server -p cortex-cli",
            "HEALTHCHECK --interval=30s",
        ],
        failures,
    )


def check_compose(failures: list[str]) -> dict[str, object]:
    return require_markers(
        "docker-compose.yml",
        [
            "cortexdb-seed:",
            "image: cortexdb:local",
            "./examples/datasets/investment_projects:/fixtures/investment_projects:ro",
            "cortexdb load-fixture /data /fixtures/investment_projects",
            "condition: service_completed_successfully",
            "CORTEXDB_DASHBOARD: \"true\"",
            "curl\", \"-sf\", \"http://localhost:8181/v1/health\"",
            '"8181:8181"',
        ],
        failures,
    )


def check_docs(failures: list[str]) -> dict[str, object]:
    checked = [
        require_markers(
            "docs/DOCKER.md",
            [
                "ghcr.io/aubakirovarman/cortexdb:latest",
                "docker compose up --build -d",
                "cortexdb-seed",
                "open http://127.0.0.1:8181/dashboard",
                "make docker-quickstart-check",
            ],
            failures,
        ),
        require_markers(
            "docs/GETTING_STARTED.md",
            [
                "Optional Docker Server Path",
                "docker compose up --build -d",
                "ghcr.io/aubakirovarman/cortexdb:latest",
                "make docker-quickstart-check",
            ],
            failures,
        ),
        require_markers("docs/DOCUMENTATION_INDEX.md", ["DOCKER.md"], failures),
    ]
    return {"docs_checked": checked}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/docker-quickstart/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    report = {
        "schema_version": 1,
        "release_workflow": check_release_workflow(failures),
        "dockerfile": check_dockerfile(failures),
        "compose": check_compose(failures),
        "docs": check_docs(failures),
        "contract": {
            "image": "ghcr.io/aubakirovarman/cortexdb",
            "server_port": 8181,
            "fixture": "examples/datasets/investment_projects",
            "dashboard": "/dashboard",
        },
        "failures": failures,
    }
    report["status"] = "failed" if failures else "passed"
    output = ROOT / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"docker quickstart check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"docker quickstart check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
