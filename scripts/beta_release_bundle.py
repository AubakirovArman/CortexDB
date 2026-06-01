#!/usr/bin/env python3
"""Run and package the CortexDB beta release evidence matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tarfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


VERSION = "0.2.0-beta.1"

SUITES: tuple[dict[str, Any], ...] = (
    {"name": "beta_foundation", "command": ["make", "beta-foundation-check"]},
    {"name": "sdk_e2e_release", "command": ["make", "sdk-e2e-release-check"]},
    {"name": "context_pack_quality", "command": ["make", "context-pack-quality-check"]},
    {"name": "verification_quality", "command": ["make", "verification-quality-check"]},
    {"name": "retrieval_quality", "command": ["make", "retrieval-quality-check"]},
    {"name": "openapi_contract", "command": ["make", "openapi-contract-check"]},
    {"name": "sdk_contract", "command": ["make", "sdk-contract-check"]},
    {"name": "security_hardening", "command": ["make", "security-hardening-check"]},
    {"name": "tenant_recovery", "command": ["make", "tenant-recovery-check"]},
    {"name": "backup_drill", "command": ["make", "backup-drill-check"]},
    {
        "name": "binary_release",
        "command": [
            "make",
            "binary-release-check",
            "BINARY_RELEASE_VERSION=v0.2.0-beta.1",
            "BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.1-local",
        ],
    },
    {"name": "rag_demo_smoke", "command": ["make", "rag-demo-smoke"]},
    {"name": "public_claims", "command": ["make", "public-claims-check"]},
    {"name": "beta_delta", "command": ["make", "beta-delta-check"]},
)

KNOWN_ARTIFACTS = (
    "target/beta-foundation/report.json",
    "target/sdk-e2e-release/report.json",
    "target/sdk-registry-gate/report.json",
    "target/context-pack-quality/report.json",
    "target/verification-quality/report.json",
    "target/retrieval-quality/report.json",
    "target/retrieval-quality/beta-report.json",
    "target/retrieval-quality/dashboard.html",
    "target/security/report.json",
    "target/security-hardening/report.json",
    "target/tenant-recovery/report.json",
    "target/backup-drill/report.json",
    "target/binary-platform-matrix/report.json",
    "target/release-artifacts/cortexdb-v0.2.0-beta.1-local.tar.gz",
    "target/release-artifacts/cortexdb-v0.2.0-beta.1-local.tar.gz.sha256",
    "target/public-claims/report.json",
)


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def git_sha(repo: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def run_suite(repo: Path, root: Path, suite: dict[str, Any]) -> dict[str, Any]:
    log_path = root / "logs" / f"{suite['name']}.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    started_at = utc_now()
    result = subprocess.run(
        suite["command"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )
    output = result.stdout
    if result.stderr:
        output += "\n--- stderr ---\n" + result.stderr
    log_path.write_text(output, encoding="utf-8")
    return {
        "name": suite["name"],
        "status": "passed" if result.returncode == 0 else "failed",
        "exit_code": result.returncode,
        "command": suite["command"],
        "started_at": started_at,
        "finished_at": utc_now(),
        "log": str(log_path),
    }


def copy_artifact_manifest(repo: Path, root: Path) -> list[str]:
    found: list[str] = []
    for relative in KNOWN_ARTIFACTS:
        path = repo / relative
        if path.exists():
            found.append(relative)
    manifest = {
        "schema_version": "cortexdb.beta_release.artifacts.v1",
        "artifacts": found,
    }
    artifact_manifest = root / "artifacts.json"
    artifact_manifest.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return found


def write_archive(repo: Path, root: Path, archive_path: Path, report_path: Path) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive_path, "w:gz") as archive:
        archive.add(report_path, arcname="report.json")
        for path in root.rglob("*"):
            if path == archive_path or path.is_dir():
                continue
            archive.add(path, arcname=path.relative_to(root.parent))
        for relative in KNOWN_ARTIFACTS:
            artifact = repo / relative
            if artifact.exists():
                archive.add(artifact, arcname=relative)


def build_report(repo: Path, root: Path, report_path: Path, archive_path: Path) -> dict[str, Any]:
    started_at = utc_now()
    suites = [run_suite(repo, root, suite) for suite in SUITES]
    artifact_paths = copy_artifact_manifest(repo, root)
    status = "passed" if all(suite["status"] == "passed" for suite in suites) else "failed"
    report = {
        "schema_version": "cortexdb.beta_release.report.v1",
        "version": VERSION,
        "status": status,
        "git_sha": git_sha(repo),
        "started_at": started_at,
        "finished_at": utc_now(),
        "suites": suites,
        "artifacts": {
            "root": str(root),
            "report": str(report_path),
            "archive": str(archive_path),
            "included_reports": artifact_paths,
        },
        "boundary": {
            "proves": "local single-node developer/API beta readiness evidence",
            "does_not_prove": [
                "production distributed consensus",
                "managed cloud readiness",
                "enterprise compliance",
                "legal-grade verification",
                "unrestricted HNSW without fallback",
                "built-in production LLM inference",
            ],
        },
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_archive(repo, root, archive_path, report_path)
    return report


def self_test() -> int:
    names = [suite["name"] for suite in SUITES]
    if len(names) != len(set(names)):
        print("beta release self-test failed: duplicate suite names")
        return 1
    required = {"sdk_e2e_release", "context_pack_quality", "verification_quality", "retrieval_quality"}
    missing = sorted(required.difference(names))
    if missing:
        print(f"beta release self-test failed: missing suites {missing}")
        return 1
    print("beta release self-test passed")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/beta-release")
    parser.add_argument("--report", default="target/beta-release/report.json")
    parser.add_argument("--archive", default="target/beta-release/evidence.tar.gz")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    repo = repo_root()
    root = repo / args.root
    report_path = repo / args.report
    archive_path = repo / args.archive
    root.mkdir(parents=True, exist_ok=True)
    report = build_report(repo, root, report_path, archive_path)
    if report["status"] != "passed":
        print(f"beta release check failed; see {report_path}", file=sys.stderr)
        return 1
    print(f"beta release check passed: {report_path}")
    print(f"beta evidence archive: {archive_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
