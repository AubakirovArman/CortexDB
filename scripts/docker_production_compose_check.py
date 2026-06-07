#!/usr/bin/env python3
"""Validate the production Docker Compose example contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


COMPOSE_FILE = Path("docker-compose.production.yml")
NGINX_CONF = Path("docs/deployment/nginx/cortexdb.conf")
AUTH_EXAMPLE = Path("docs/deployment/auth.tokens.example")
DOCKER_DOC = Path("docs/DOCKER.md")
DOC_INDEX = Path("docs/DOCUMENTATION_INDEX.md")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def check_compose(failures: list[str]) -> dict[str, object]:
    text = read(COMPOSE_FILE)
    markers = [
        "reverse-proxy:",
        "nginx:1.27-alpine",
        '"8181:8080"',
        "./docs/deployment/nginx/cortexdb.conf:/etc/nginx/nginx.conf:ro",
        "CORTEXDB_AUTH_TOKENS_FILE: /run/secrets/cortexdb-auth.tokens",
        "./secrets/auth.tokens:/run/secrets/cortexdb-auth.tokens:ro",
        "CORTEXDB_RATE_LIMIT_PER_MINUTE: \"6000\"",
        "CORTEXDB_AUDIT_LOG_FILE: /data/audit.jsonl",
        "cortexdb-data:/data:rw",
        "backup-sidecar:",
        "profiles:",
        "- maintenance",
        "cortexdb-backups:/backups:rw",
        "cortexdb backup /data /backups/cortexdb-$$stamp",
        "cortexdb validate /backups/cortexdb-$$stamp",
    ]
    for marker in markers:
        require(marker in text, f"{COMPOSE_FILE} missing {marker!r}", failures)
    require("CORTEXDB_AUTH_TOKEN:" not in text, "production compose must use token file auth", failures)
    require("privileged: true" not in text, "production compose must not enable privileged mode", failures)
    return {"path": str(COMPOSE_FILE), "markers_checked": len(markers)}


def check_nginx(failures: list[str]) -> dict[str, object]:
    text = read(NGINX_CONF)
    markers = [
        "server_tokens off;",
        "client_max_body_size 2m;",
        "server cortexdb:8181;",
        "listen 8080;",
        "proxy_set_header Authorization $http_authorization;",
        "proxy_pass http://cortexdb_backend;",
    ]
    for marker in markers:
        require(marker in text, f"{NGINX_CONF} missing {marker!r}", failures)
    return {"path": str(NGINX_CONF), "markers_checked": len(markers)}


def check_auth_example(failures: list[str]) -> dict[str, object]:
    text = read(AUTH_EXAMPLE)
    markers = [
        "Format: role:token[:agent_id]",
        "admin:replace-with-random-admin-token",
        "data:replace-with-random-data-token",
    ]
    for marker in markers:
        require(marker in text, f"{AUTH_EXAMPLE} missing {marker!r}", failures)
    return {"path": str(AUTH_EXAMPLE), "markers_checked": len(markers)}


def check_docs(failures: list[str]) -> dict[str, object]:
    docs = {
        DOCKER_DOC: [
            "docker-compose.production.yml",
            "reverse-proxy",
            "CORTEXDB_AUTH_TOKENS_FILE",
            "backup-sidecar",
            "make docker-production-compose-check",
        ],
        DOC_INDEX: ["docker-compose.production.yml", "auth.tokens.example", "cortexdb.conf"],
    }
    checked: list[str] = []
    for path, markers in docs.items():
        text = read(path)
        checked.append(str(path))
        for marker in markers:
            require(marker in text, f"{path} missing {marker!r}", failures)
    return {"docs_checked": checked}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/docker-production-compose/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    report = {
        "schema_version": 1,
        "compose": check_compose(failures),
        "nginx": check_nginx(failures),
        "auth_example": check_auth_example(failures),
        "docs": check_docs(failures),
        "contract": {
            "reverse_proxy": "nginx",
            "external_port": 8181,
            "backend_port": 8181,
            "auth": "CORTEXDB_AUTH_TOKENS_FILE",
            "data_volume": "cortexdb-data",
            "backup_volume": "cortexdb-backups",
            "backup_profile": "maintenance",
        },
        "failures": failures,
    }
    report["status"] = "failed" if failures else "passed"

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"docker production compose check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"docker production compose check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
