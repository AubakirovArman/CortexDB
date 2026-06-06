#!/usr/bin/env python3
"""Build a unified CortexDB release evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tarfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


PASSED_STATUSES = {"ok", "passed"}


@dataclass(frozen=True)
class ArtifactSpec:
    name: str
    category: str
    path: str
    required: bool = True
    validate_report: bool = True


REQUIRED_ARTIFACTS: tuple[ArtifactSpec, ...] = (
    ArtifactSpec("production_evidence", "release", "target/production-evidence/report.json"),
    ArtifactSpec("release_artifact_manifest", "release", "target/release-artifact-manifest/manifest.json", validate_report=False),
    ArtifactSpec("release_artifact_manifest_report", "release", "target/release-artifact-manifest/report.json"),
    ArtifactSpec("sdk_e2e_release", "sdk", "target/sdk-e2e-release/report.json"),
    ArtifactSpec("sdk_release_artifacts", "sdk", "target/sdk-release-artifacts/report.json"),
    ArtifactSpec("sdk_registry_gate", "sdk", "target/sdk-registry-gate/report.json"),
    ArtifactSpec("context_pack_quality", "benchmark", "target/context-pack-quality/report.json"),
    ArtifactSpec("verification_quality", "benchmark", "target/verification-quality/report.json"),
    ArtifactSpec("retrieval_quality", "benchmark", "target/retrieval-quality/report.json"),
    ArtifactSpec("single_node_performance", "benchmark", "target/single-node-performance/report.json"),
    ArtifactSpec("security_hardening", "security", "target/security-hardening/report.json"),
    ArtifactSpec("public_claims", "security", "target/public-claims/report.json"),
    ArtifactSpec("backup_drill", "storage", "target/backup-drill/report.json"),
    ArtifactSpec("backup_offsite", "storage", "target/backup-offsite/report.json"),
    ArtifactSpec("backup_restore_pack", "storage", "target/backup-restore-production-pack/report.json"),
    ArtifactSpec("tenant_recovery", "operations", "target/tenant-recovery/report.json"),
)

OPTIONAL_ARTIFACTS: tuple[ArtifactSpec, ...] = (
    ArtifactSpec("dashboard_package", "ui", "target/dashboard/dashboard-v1.tar.gz", required=False, validate_report=False),
    ArtifactSpec("retrieval_dashboard", "benchmark", "target/retrieval-quality/dashboard.html", required=False, validate_report=False),
    ArtifactSpec("ann_release_history", "benchmark", "target/ann/release-evidence/corpus-runs/history.json", required=False, validate_report=False),
    ArtifactSpec("ann_real_embedding_history", "benchmark", "target/ann/real-embedding/runs/history.json", required=False, validate_report=False),
    ArtifactSpec("chaos_restart", "storage", "target/chaos-restart/report.json", required=False),
    ArtifactSpec("crash_fault", "storage", "target/crash-fault/report.json", required=False),
    ArtifactSpec("replication_lifecycle", "experimental", "target/replication-lifecycle/report.json", required=False),
)


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def git_value(repo: Path, args: list[str]) -> str:
    result = subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def report_passed(data: dict[str, Any]) -> bool:
    status = data.get("status")
    if isinstance(status, str):
        return status.lower() in PASSED_STATUSES
    if data.get("ok") is True:
        errors = data.get("errors")
        return not errors
    return False


def artifact_entry(repo: Path, spec: ArtifactSpec) -> tuple[dict[str, Any] | None, list[str]]:
    path = repo / spec.path
    if not path.is_file():
        if spec.required:
            return None, [f"missing required artifact: {spec.path}"]
        return None, []

    failures: list[str] = []
    status: str | None = None
    schema_version: Any = None
    if spec.validate_report:
        try:
            data = read_json(path)
        except Exception as error:  # noqa: BLE001 - report validation must be structured.
            failures.append(f"{spec.path}: invalid JSON report: {error}")
        else:
            schema_version = data.get("schema_version")
            raw_status = data.get("status")
            status = str(raw_status) if raw_status is not None else ("ok" if data.get("ok") is True else None)
            if not report_passed(data):
                failures.append(f"{spec.path}: expected passed/ok report")

    entry = {
        "name": spec.name,
        "category": spec.category,
        "path": spec.path,
        "size_bytes": path.stat().st_size,
        "sha256": sha256_file(path),
        "required": spec.required,
        "validate_report": spec.validate_report,
    }
    if status is not None:
        entry["status"] = status
    if schema_version is not None:
        entry["schema_version"] = schema_version
    return entry, failures


def collect_artifacts(repo: Path, binary_archive: str | None) -> tuple[list[dict[str, Any]], list[str]]:
    specs = list(REQUIRED_ARTIFACTS)
    if binary_archive:
        specs.append(ArtifactSpec("binary_archive", "release", binary_archive, validate_report=False))
        sidecar = Path(binary_archive + ".sha256")
        specs.append(ArtifactSpec("binary_archive_sha256", "release", sidecar.as_posix(), validate_report=False))
    specs.extend(OPTIONAL_ARTIFACTS)

    artifacts: list[dict[str, Any]] = []
    failures: list[str] = []
    seen_paths: set[str] = set()
    for spec in specs:
        if spec.path in seen_paths:
            continue
        seen_paths.add(spec.path)
        entry, entry_failures = artifact_entry(repo, spec)
        failures.extend(entry_failures)
        if entry is not None:
            artifacts.append(entry)
    return artifacts, failures


def write_archive(repo: Path, archive_path: Path, manifest_path: Path, report_path: Path, artifacts: list[dict[str, Any]]) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive_path, "w:gz") as archive:
        archive.add(manifest_path, arcname="release-evidence/manifest.json")
        archive.add(report_path, arcname="release-evidence/report.json")
        for artifact in artifacts:
            relative = artifact["path"]
            path = repo / relative
            if not path.is_file():
                continue
            archive.add(path, arcname=f"release-evidence/artifacts/{relative}")


def build_bundle(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    root = repo / args.root
    manifest_path = repo / args.manifest
    report_path = repo / args.report
    archive_path = repo / args.archive
    root.mkdir(parents=True, exist_ok=True)

    artifacts, failures = collect_artifacts(repo, args.binary_archive)
    artifact_count_by_category: dict[str, int] = {}
    for artifact in artifacts:
        category = str(artifact["category"])
        artifact_count_by_category[category] = artifact_count_by_category.get(category, 0) + 1

    manifest = {
        "schema_version": "cortexdb.release_evidence_bundle.v1",
        "created_at": utc_now(),
        "git": {
            "commit": git_value(repo, ["rev-parse", "HEAD"]),
            "branch": git_value(repo, ["rev-parse", "--abbrev-ref", "HEAD"]),
            "dirty": bool(git_value(repo, ["status", "--short"])),
        },
        "boundary": {
            "proves": "local single-node release evidence bundle integrity",
            "does_not_prove": [
                "production distributed consensus",
                "managed cloud readiness",
                "enterprise compliance certification",
                "legal-grade verification",
                "unrestricted HNSW without fallback",
            ],
        },
        "artifact_count": len(artifacts),
        "artifact_count_by_category": artifact_count_by_category,
        "artifacts": artifacts,
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    report = {
        "schema_version": "cortexdb.release_evidence_bundle_report.v1",
        "status": "passed" if not failures else "failed",
        "manifest": args.manifest,
        "archive": args.archive,
        "artifact_count": len(artifacts),
        "artifact_count_by_category": artifact_count_by_category,
        "failures": failures,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not failures:
        write_archive(repo, archive_path, manifest_path, report_path, artifacts)
        archive_sha = sha256_file(archive_path)
        sidecar = archive_path.with_suffix(archive_path.suffix + ".sha256")
        sidecar.write_text(f"{archive_sha}  {archive_path.name}\n", encoding="utf-8")
        report["archive_sha256"] = archive_sha
        report["archive_sha256_sidecar"] = str(sidecar.relative_to(repo))
        report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def self_test() -> int:
    names = [spec.name for spec in REQUIRED_ARTIFACTS]
    if len(names) != len(set(names)):
        print("release evidence bundle self-test failed: duplicate artifact names", file=sys.stderr)
        return 1
    required_categories = {"release", "sdk", "benchmark", "security", "storage"}
    categories = {spec.category for spec in REQUIRED_ARTIFACTS}
    missing = sorted(required_categories.difference(categories))
    if missing:
        print(f"release evidence bundle self-test failed: missing categories {missing}", file=sys.stderr)
        return 1
    if not report_passed({"status": "passed"}) or not report_passed({"status": "ok"}) or not report_passed({"ok": True, "errors": []}):
        print("release evidence bundle self-test failed: pass status detection", file=sys.stderr)
        return 1
    print("release evidence bundle self-test passed")
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="target/release-evidence-bundle")
    parser.add_argument("--manifest", default="target/release-evidence-bundle/manifest.json")
    parser.add_argument("--report", default="target/release-evidence-bundle/report.json")
    parser.add_argument("--archive", default="target/release-evidence-bundle/release-evidence.tar.gz")
    parser.add_argument("--binary-archive", default="target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        return self_test()
    report = build_bundle(args)
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"release evidence bundle check passed: {repo_root() / args.report}")
    print(f"release evidence archive: {repo_root() / args.archive}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
