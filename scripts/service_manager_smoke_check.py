#!/usr/bin/env python3
"""Validate systemd and launchd operator examples."""

from __future__ import annotations

import argparse
import json
import plistlib
from pathlib import Path


SYSTEMD_SERVICE = Path("docs/deployment/cortexdb.service")
LAUNCHD_PLIST = Path("docs/deployment/com.cortexdb.server.plist")


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def check_systemd(failures: list[str]) -> dict[str, object]:
    text = read(SYSTEMD_SERVICE)
    markers = [
        "[Unit]",
        "[Service]",
        "[Install]",
        "ExecStart=/usr/local/bin/cortex-server /var/lib/cortexdb 127.0.0.1:8181",
        "EnvironmentFile=/etc/cortexdb/cortexdb.env",
        "StandardOutput=journal",
        "StandardError=journal",
        "Restart=on-failure",
        "NoNewPrivileges=true",
        "ProtectSystem=strict",
        "ReadWritePaths=/var/lib/cortexdb",
    ]
    for marker in markers:
        require(marker in text, f"systemd service missing {marker!r}", failures)
    require("0.0.0.0:8181" not in text, "systemd service should not bind all interfaces by default", failures)
    return {"path": str(SYSTEMD_SERVICE), "markers_checked": len(markers)}


def check_launchd(failures: list[str]) -> dict[str, object]:
    with LAUNCHD_PLIST.open("rb") as handle:
        plist = plistlib.load(handle)

    args = plist.get("ProgramArguments", [])
    env = plist.get("EnvironmentVariables", {})
    require(plist.get("Label") == "com.cortexdb.server", "launchd label mismatch", failures)
    require(args == ["/usr/local/bin/cortex-server", "/usr/local/var/cortexdb", "127.0.0.1:8181"], "launchd ProgramArguments mismatch", failures)
    require(plist.get("WorkingDirectory") == "/usr/local/var/cortexdb", "launchd WorkingDirectory mismatch", failures)
    require(plist.get("RunAtLoad") is True, "launchd RunAtLoad must be true", failures)
    require(plist.get("KeepAlive") is True, "launchd KeepAlive must be true", failures)
    require(env.get("CORTEXDB_AUTH_TOKENS_FILE") == "/usr/local/etc/cortexdb/auth.tokens", "launchd auth token env missing", failures)
    require(env.get("CORTEXDB_ACTOR_QUEUE_CAPACITY") == "1024", "launchd actor queue env missing", failures)
    require(env.get("CORTEXDB_RATE_LIMIT_PER_MINUTE") == "6000", "launchd rate limit env missing", failures)
    require(env.get("CORTEXDB_AUDIT_LOG_FILE") == "/usr/local/var/log/cortexdb/audit.jsonl", "launchd audit log env missing", failures)
    require(plist.get("StandardOutPath") == "/usr/local/var/log/cortexdb/server.log", "launchd stdout log path mismatch", failures)
    require(plist.get("StandardErrorPath") == "/usr/local/var/log/cortexdb/server.error.log", "launchd stderr log path mismatch", failures)
    return {"path": str(LAUNCHD_PLIST), "program_arguments": args}


def check_docs(failures: list[str]) -> dict[str, object]:
    docs = {
        "docs/archive/SYSTEMD.md": [
            "systemctl enable --now cortexdb",
            "/v1/validate",
            "Environment File",
            "journalctl -u cortexdb",
            "CORTEXDB_AUDIT_LOG_FILE=/var/lib/cortexdb/audit.jsonl",
        ],
        "docs/archive/LAUNCHD.md": [
            "launchctl bootstrap",
            "launchctl kickstart",
            "launchctl bootout",
            "/v1/validate",
            "CORTEXDB_ACTOR_QUEUE_CAPACITY=1024",
            "CORTEXDB_RATE_LIMIT_PER_MINUTE=6000",
            "CORTEXDB_AUDIT_LOG_FILE=/usr/local/var/log/cortexdb/audit.jsonl",
            "StandardOutPath=/usr/local/var/log/cortexdb/server.log",
            "StandardErrorPath=/usr/local/var/log/cortexdb/server.error.log",
        ],
        "docs/DOCUMENTATION_INDEX.md": ["LAUNCHD.md", "SYSTEMD.md"],
        "docs/OPERATIONS.md": ["SYSTEMD.md", "LAUNCHD.md"],
    }
    checked = []
    for file_name, markers in docs.items():
        text = read(Path(file_name))
        checked.append(file_name)
        for marker in markers:
            require(marker in text, f"{file_name} missing {marker!r}", failures)
    return {"docs_checked": checked}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/service-manager-smoke/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    report = {
        "schema_version": 1,
        "systemd": check_systemd(failures),
        "launchd": check_launchd(failures),
        "docs": check_docs(failures),
        "failures": failures,
    }
    report["status"] = "failed" if failures else "passed"

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"service manager smoke check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"service manager smoke check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
