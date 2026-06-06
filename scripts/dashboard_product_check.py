#!/usr/bin/env python3
"""Validate Dashboard Product UI evidence wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "read_only_mode": [
        ("web/dashboard/src/index.html", "id=\"read-only-mode\""),
        ("web/dashboard/src/app.js", "guardWriteAllowed"),
        ("web/dashboard/src/app.js", "cortexdb-dashboard-read-only"),
    ],
    "operational_status": [
        ("web/dashboard/src/index.html", "id=\"status-report\""),
        ("web/dashboard/src/index.html", "backup posture"),
        ("web/dashboard/src/app.js", "dashboard_status.v1"),
        ("web/dashboard/src/app.js", "/v1/compatibility"),
        ("web/dashboard/src/app.js", "summarizeCompatibilityResult"),
        ("web/dashboard/src/app.js", "backup_posture"),
        ("web/dashboard/src/app.js", "actor_queue_depth"),
        ("web/dashboard/src/app.js", "actor_queue_capacity"),
        ("web/dashboard/src/app.js", "backup_latest_age_seconds"),
        ("web/dashboard/src/app.js", "last_request_error"),
        ("web/dashboard/src/app.js", "make backup-restore-production-pack-check"),
        ("web/dashboard/src/reporting_operations.js", "renderOperationalStatus"),
        ("web/dashboard/src/reporting_operations.js", "Version compatibility"),
        ("web/dashboard/src/reporting_operations.js", "API / SDK / storage / migration"),
        ("web/dashboard/src/reporting_operations.js", "Backup posture"),
        ("web/dashboard/src/reporting_operations.js", "Actor queue"),
        ("web/dashboard/src/reporting_operations.js", "Latest backup"),
        ("web/dashboard/src/reporting_operations.js", "Last error"),
        ("docs/DASHBOARD_UI.md", "Operational Status View"),
    ],
    "incident_timeline": [
        ("web/dashboard/src/app.js", "incident_timeline"),
        ("web/dashboard/src/app.js", "buildIncidentTimeline"),
        ("web/dashboard/src/app.js", "audit_event"),
        ("web/dashboard/src/app.js", "rate_limit_event"),
        ("web/dashboard/src/app.js", "storage_event"),
        ("web/dashboard/src/app.js", "backup_event"),
        ("web/dashboard/src/reporting_operations.js", "renderIncidentEvent"),
        ("web/dashboard/src/reporting_operations.js", "Incident timeline"),
        ("web/dashboard/src/reporting_operations.js", "audit / rate / storage / backup"),
        ("docs/DASHBOARD_UI.md", "Incident Timeline"),
    ],
    "single_node_slo_dashboard": [
        ("web/dashboard/src/index.html", "id=\"slo-report\""),
        ("web/dashboard/src/index.html", "Single-node SLO dashboard"),
        ("web/dashboard/src/app.js", "dashboard_slo.v1"),
        ("web/dashboard/src/app.js", "buildSloDashboard"),
        ("web/dashboard/src/app.js", "backup_freshness"),
        ("web/dashboard/src/app.js", "validation_status"),
        ("web/dashboard/src/app.js", "error_budget"),
        ("web/dashboard/src/reporting_slo.js", "renderSloDashboard"),
        ("web/dashboard/src/reporting_slo.js", "Availability"),
        ("web/dashboard/src/reporting_slo.js", "Latency"),
        ("web/dashboard/src/reporting_slo.js", "Backup freshness"),
        ("web/dashboard/src/reporting_slo.js", "Validation status"),
        ("web/dashboard/src/reporting_slo.js", "Error budget"),
        ("docs/DASHBOARD_UI.md", "Single-node SLO Dashboard"),
        ("docs/SINGLE_NODE_SLO.md", "dashboard_slo.v1"),
    ],
    "audit_readiness": [
        ("web/dashboard/src/index.html", "id=\"audit-report\""),
        ("web/dashboard/src/index.html", "data-action=\"audit-readiness\""),
        ("web/dashboard/src/reporting_audit.js", "dashboard_audit_readiness.v1"),
        ("web/dashboard/src/reporting_audit.js", "renderAuditReadiness"),
        ("docs/DASHBOARD_UI.md", "Audit readiness"),
    ],
    "permissions_view": [
        ("web/dashboard/src/index.html", "href=\"/dashboard/permissions\""),
        ("web/dashboard/src/dashboard_manifest.json", "\"permissions\""),
        ("web/dashboard/src/reporting_operations.js", "renderPermissionsView"),
        ("web/dashboard/src/app.js", "selected_scopes"),
        ("web/dashboard/src/app.js", "server_token_policy"),
        ("web/dashboard/src/app.js", "anonymous_synthetic_view"),
        ("web/dashboard/src/reporting_operations.js", "Permissions explorer"),
        ("web/dashboard/src/reporting_operations.js", "Token / role / scope / AgentView"),
        ("web/dashboard/src/reporting_operations.js", "Scope probes"),
        ("web/dashboard/src/reporting_operations.js", "AgentView policy"),
        ("docs/DASHBOARD_UI.md", "Permissions Explorer"),
    ],
    "context_pack_explorer": [
        ("web/dashboard/src/index.html", "id=\"context-report\""),
        ("web/dashboard/src/reporting_retrieval.js", "Citation explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Explain explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Anomaly explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Token budget"),
        ("web/dashboard/src/reporting_retrieval.js", "Source refs"),
        ("web/dashboard/src/reporting_retrieval.js", "score_components"),
        ("web/dashboard/src/reporting_retrieval.js", "why_excluded"),
        ("docs/DASHBOARD_UI.md", "ContextPack Explorer"),
    ],
    "verification_explorer": [
        ("web/dashboard/src/index.html", "id=\"verify-report\""),
        ("web/dashboard/src/reporting_retrieval.js", "Mixed evidence"),
        ("web/dashboard/src/reporting_retrieval.js", "Contradicting evidence"),
        ("web/dashboard/src/reporting_retrieval.js", "Numeric conflict explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Guard explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "numeric_conflicts"),
        ("docs/DASHBOARD_UI.md", "Verification Explorer"),
    ],
    "ingestion_job_dashboard": [
        ("web/dashboard/src/index.html", "id=\"ingest-jobs-list-button\""),
        ("web/dashboard/src/app.js", "/v1/ingest/jobs"),
        ("web/dashboard/src/reporting_ingest.js", "ingestionJobDashboard"),
        ("web/dashboard/src/reporting_ingest.js", "progress failures warnings records chunks source refs"),
        ("web/dashboard/src/reporting_ingest.js", "Ingestion job records"),
        ("web/dashboard/src/reporting_ingest.js", "failure reason"),
        ("web/dashboard/src/reporting_ingest.js", "Ingestion chunks and SourceRefs"),
        ("web/dashboard/src/style.css", ".report-table"),
        ("docs/DASHBOARD_UI.md", "Ingestion Job Dashboard"),
    ],
    "release_artifacts": [
        ("e2e/dashboard_screenshots.mjs", "permissions"),
        ("docs/DASHBOARD_UI.md", "dashboard-screenshots"),
        ("docs/DASHBOARD_PRODUCT_UI_EVIDENCE.md", "make dashboard-product-check"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok

    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "checks": checks,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except RuntimeError as error:
        print(f"dashboard product check failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"dashboard product check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
