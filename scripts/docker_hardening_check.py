#!/usr/bin/env python3
"""Validate Docker hardening docs and local container contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DOCKERFILE = Path("Dockerfile")
COMPOSE_FILE = Path("docker-compose.yml")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def check_dockerfile(failures: list[str]) -> dict[str, object]:
    text = read(DOCKERFILE)
    markers = [
        "FROM rust:1-bookworm AS build",
        "FROM debian:bookworm-slim",
        "groupadd --system --gid 10001 cortexdb",
        "useradd --system --uid 10001 --gid 10001",
        "install -d -o cortexdb -g cortexdb -m 0750 /data",
        "USER 10001:10001",
        'VOLUME ["/data"]',
        "HEALTHCHECK --interval=30s",
        "curl -sf http://localhost:8181/v1/health",
    ]
    for marker in markers:
        require(marker in text, f"Dockerfile missing {marker!r}", failures)
    require("USER root" not in text, "Dockerfile must not switch back to root", failures)
    return {"path": str(DOCKERFILE), "markers_checked": len(markers)}


def check_compose(failures: list[str]) -> dict[str, object]:
    text = read(COMPOSE_FILE)
    markers = [
        'user: "10001:10001"',
        "read_only: true",
        "cortexdb-data:/data:rw",
        "/tmp:rw,noexec,nosuid,size=64m",
        "no-new-privileges:true",
        "cap_drop:",
        "- ALL",
        'test: ["CMD", "curl", "-sf", "http://localhost:8181/v1/health"]',
    ]
    for marker in markers:
        require(marker in text, f"docker-compose.yml missing {marker!r}", failures)
    require("privileged: true" not in text, "docker-compose.yml must not enable privileged mode", failures)
    return {"path": str(COMPOSE_FILE), "markers_checked": len(markers)}


def check_docs(failures: list[str]) -> dict[str, object]:
    docs = {
        "docs/DOCKER.md": [
            "runtime user: 10001:10001",
            "read_only: true",
            "tmpfs: /tmp:rw,noexec,nosuid,size=64m",
            "security_opt: no-new-privileges:true",
            "cap_drop: ALL",
            "cortexdb-data:/data:rw",
            "sudo chown 10001:10001 ./data",
            "make docker-hardening-check",
        ],
        "docs/DOCUMENTATION_INDEX.md": ["DOCKER.md"],
        "docs/SDK_DOCKER_OBSERVABILITY.md": ["DOCKER.md"],
    }
    checked: list[str] = []
    for file_name, markers in docs.items():
        text = read(Path(file_name))
        checked.append(file_name)
        for marker in markers:
            require(marker in text, f"{file_name} missing {marker!r}", failures)
    return {"docs_checked": checked}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/docker-hardening/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    report = {
        "schema_version": 1,
        "dockerfile": check_dockerfile(failures),
        "compose": check_compose(failures),
        "docs": check_docs(failures),
        "hardening": {
            "non_root_uid_gid": "10001:10001",
            "read_only_root": True,
            "writable_volume": "/data",
            "tmpfs": ["/tmp"],
            "no_new_privileges": True,
            "cap_drop_all": True,
            "healthcheck": "http://localhost:8181/v1/health",
        },
        "failures": failures,
    }
    report["status"] = "failed" if failures else "passed"

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"docker hardening check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"docker hardening check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
