#!/usr/bin/env python3
"""Validate the Ingestion Jobs v2 evidence surface."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


REQUIREMENTS: dict[str, dict[str, object]] = {
    "durable_jobs": {
        "files": {
            "crates/cortex-engine/src/ingestion/jobs.rs": [
                "pub fn save_ingestion_job",
                "pub fn load_ingestion_job",
                "pub fn list_ingestion_jobs",
                "write_atomic",
                "file.sync_all()",
                "sync_all()?",
            ],
            "crates/cortex-engine/tests/ingestion_job_tests.rs": [
                "ingestion_job_durable_save_and_load",
                "ingestion_job_list_roundtrip",
            ],
        },
    },
    "progress": {
        "files": {
            "crates/cortex-engine/src/ingestion/progress.rs": [
                "total_items",
                "completed_items",
                "failed_items",
                "last_cell_id",
                "record_cell",
                "finish",
            ],
            "crates/cortex-server/src/tests/ingest_tests.rs": [
                "\"completed_items\":2",
                "\"last_cell_id\":10002",
            ],
        },
    },
    "failure_reasons": {
        "files": {
            "crates/cortex-engine/src/ingestion/progress.rs": [
                "pub message: Option<String>",
                "progress.message = Some(message.into())",
            ],
            "crates/cortex-engine/tests/ingestion_job_tests.rs": [
                "Some(\"parse error\".to_owned())",
                "Some(\"boom\".to_owned())",
            ],
        },
    },
    "retry": {
        "files": {
            "crates/cortex-engine/src/ingestion/jobs.rs": [
                "pub fn retry_ingestion_job",
                "retry_count",
                "max_retries",
            ],
            "crates/cortex-server/src/router.rs": [
                "path.ends_with(\"/retry\")",
                "db.retry_ingestion_job(id)?",
            ],
            "crates/cortex-cli/src/cli_ingest.rs": ["retry_ingestion_job"],
        },
    },
    "cancel": {
        "files": {
            "crates/cortex-engine/src/ingestion/jobs.rs": ["pub fn cancel_ingestion_job"],
            "crates/cortex-server/src/router.rs": [
                "path.ends_with(\"/cancel\")",
                "db.cancel_ingestion_job(id)?",
            ],
            "crates/cortex-cli/src/cli_ingest.rs": ["cancel_ingestion_job"],
        },
    },
    "resume_after_restart": {
        "files": {
            "crates/cortex-engine/src/ingestion/jobs.rs": [
                "pub fn resume_interrupted_ingestion_jobs",
                "IngestionJobStatus::Running",
                "IngestionJobStatus::Queued",
                "resumed after database restart",
            ],
            "crates/cortex-engine/tests/ingestion_job_tests.rs": [
                "ingestion_job_running_requeues_after_database_reopen",
                "resume_interrupted_ingestion_jobs_returns_requeued_jobs",
            ],
        },
    },
    "operator_surfaces": {
        "files": {
            "docs/INGESTION.md": [
                "Ingestion Job Lifecycle",
                "cortexdb ingest-jobs",
                "cortexdb ingest-job-retry",
                "cortexdb ingest-job-cancel",
            ],
            "docs/API.md": [
                "GET /v1/ingest/jobs",
                "POST /v1/ingest/jobs/<job_id>/retry",
                "POST /v1/ingest/jobs/<job_id>/cancel",
                "DELETE /v1/ingest/jobs/<job_id>",
            ],
            "docs/API_JSON_SCHEMAS.md": ["GET /v1/ingest/jobs/<job_id>"],
            "docs/CLI.md": ["ingest-job-retry", "ingest-job-cancel"],
        },
    },
}


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
    if result.returncode != 0:
        return "unknown"
    return result.stdout.strip()


def read_text(repo: Path, relative: str) -> str:
    path = repo / relative
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def validate_requirement(repo: Path, name: str, spec: dict[str, object]) -> dict[str, object]:
    missing: list[str] = []
    files = spec.get("files", {})
    assert isinstance(files, dict)
    for relative, markers in files.items():
        text = read_text(repo, str(relative))
        if not text:
            missing.append(f"{relative}: missing file")
            continue
        assert isinstance(markers, list)
        for marker in markers:
            if str(marker) not in text:
                missing.append(f"{relative}: missing {marker!r}")
    return {
        "name": name,
        "status": "passed" if not missing else "failed",
        "missing": missing,
    }


def write_report(path: Path, report: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> int:
    required = {
        "durable_jobs",
        "progress",
        "failure_reasons",
        "retry",
        "cancel",
        "resume_after_restart",
    }
    missing = sorted(required.difference(REQUIREMENTS))
    if missing:
        print(f"ingestion jobs v2 self-test failed: missing {missing}")
        return 1
    print("ingestion jobs v2 self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/ingestion-jobs-v2/report.json")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    repo = repo_root()
    checks = [
        validate_requirement(repo, name, spec) for name, spec in REQUIREMENTS.items()
    ]
    status = "passed" if all(check["status"] == "passed" for check in checks) else "failed"
    report = {
        "schema_version": "cortexdb.ingestion_jobs_v2.v1",
        "status": status,
        "generated_at": utc_now(),
        "git_sha": git_sha(repo),
        "checks": checks,
        "covered_tasks": [
            "durable jobs",
            "retry",
            "cancel",
            "progress",
            "failure reasons",
            "resume after restart",
        ],
        "evidence_commands": [
            "cargo test -p cortex-engine --test ingestion_job_tests",
            "cargo test -p cortex-server ingest_tests",
            "cargo test -p cortex-cli ingest",
        ],
    }
    report_path = repo / args.report
    write_report(report_path, report)

    if status != "passed":
        print(f"ingestion jobs v2 check failed: {report_path}", file=sys.stderr)
        return 1
    print(f"ingestion jobs v2 check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
