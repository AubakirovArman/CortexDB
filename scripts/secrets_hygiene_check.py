#!/usr/bin/env python3
"""Validate E13 secrets hygiene controls."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


REPORT_SCHEMA = "cortexdb.secrets_hygiene_report.v1"
SECRET_PATTERNS = (
    re.compile(r"(?i)\bOPENAI_API_KEY\s*=\s*(?!dummy|example|placeholder|<)[A-Za-z0-9_\-]{12,}"),
    re.compile(r"(?i)\bCORTEXDB_[A-Z0-9_]*API_KEY\s*=\s*(?!dummy|example|placeholder|<)[A-Za-z0-9_\-]{12,}"),
    re.compile(r"\bsk-[A-Za-z0-9_\-]{20,}\b"),
)
EXCLUDED_PARTS = {".git", "target", "node_modules", ".venv", "venv", "__pycache__"}
MARKERS = {
    "ignore_patterns": [
        (".gitignore", ".env"),
        (".gitignore", ".env.*"),
        (".gitignore", "/secrets/"),
    ],
    "cli_no_secret_args": [
        ("crates/cortex-cli/src/cli/dispatch/paths.rs", "--passphrase is not accepted"),
        ("crates/cortex-cli/src/cli_auth_review.rs", "--tokens is not accepted"),
        ("crates/cortex-cli/src/cli_auth_review.rs", "tokens_env"),
        ("crates/cortex-cli/src/cli_auth_review_tests.rs", "auth_review_rejects_inline_tokens_argument_without_echoing_value"),
        ("crates/cortex-cli/src/tests/backup.rs", "encrypted_backup_rejects_passphrase_argument_without_echoing_value"),
    ],
    "runtime_redaction": [
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "denied_ingestion_audit_event_does_not_leak_query_body_or_token"),
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "super-secret-token"),
        ("crates/cortex-server/src/tests/security_redaction_tests.rs", "!line.contains(leaked)"),
        ("crates/cortex-server/src/audit.rs", "principal_id"),
        ("crates/cortex-server/src/audit.rs", "request_id"),
    ],
    "docs": [
        ("docs/AUTH.md", "Do not pass bearer tokens as CLI arguments"),
        ("docs/CLI.md", "command-line secrets are visible in process listings"),
        ("docs/deployment/auth.tokens.example", "replace-with-random-admin-token"),
    ],
}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def tracked_files(repo: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError("failed to list tracked files with git")
    files: list[Path] = []
    for line in result.stdout.splitlines():
        path = Path(line)
        if any(part in EXCLUDED_PARTS for part in path.parts):
            continue
        files.append(repo / path)
    return files


def validate_markers(repo: Path) -> tuple[dict[str, bool], list[str]]:
    checks: dict[str, bool] = {}
    failures: list[str] = []
    for name, markers in MARKERS.items():
        ok = True
        for file_name, marker in markers:
            try:
                text = read(repo / file_name)
            except OSError as error:
                failures.append(f"{name}: failed to read {file_name}: {error}")
                ok = False
                continue
            if marker not in text:
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return checks, failures


def validate_tracked_secret_scan(repo: Path) -> list[str]:
    failures: list[str] = []
    for path in tracked_files(repo):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        except OSError as error:
            failures.append(f"{path.relative_to(repo)}: failed to read tracked file: {error}")
            continue
        for pattern in SECRET_PATTERNS:
            if pattern.search(text):
                failures.append(f"{path.relative_to(repo)}: provider-secret-like literal detected")
                break
    return failures


def validate(repo: Path) -> dict[str, Any]:
    checks, failures = validate_markers(repo)
    secret_failures = validate_tracked_secret_scan(repo)
    failures.extend(secret_failures)
    checks["tracked_secret_scan"] = not secret_failures
    return {
        "schema_version": REPORT_SCHEMA,
        "status": "passed" if not failures else "failed",
        "checks": checks,
        "checked_markers": sum(len(markers) for markers in MARKERS.values()),
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/secrets-hygiene/report.json")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = repo_root()
    try:
        report = validate(repo)
    except Exception as error:  # noqa: BLE001 - gate should report all failures.
        print(f"secrets hygiene check failed: {error}", file=sys.stderr)
        return 1
    output = repo / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"secrets hygiene check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
