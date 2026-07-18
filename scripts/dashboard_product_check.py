#!/usr/bin/env python3
"""Validate Dashboard Product UI evidence wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path



APP_SOURCE_FILES = (
    Path("web/dashboard/src/app_state.js"),
    Path("web/dashboard/src/app_api.js"),
    Path("web/dashboard/src/app_access.js"),
    Path("web/dashboard/src/app_status.js"),
    Path("web/dashboard/src/app_incidents.js"),
    Path("web/dashboard/src/app_status_summaries.js"),
    Path("web/dashboard/src/app_slo_backup.js"),
    Path("web/dashboard/src/app_bindings.js"),
    Path("web/dashboard/src/app.js"),
)


def read_app_sources() -> str:
    return "\n".join(path.read_text(encoding="utf-8") for path in APP_SOURCE_FILES)

REQUIRED_MARKERS = {
    "read_only_mode": [
        ("web/dashboard/src/index.html", "id=\"read-only-mode\""),
        ("web/dashboard/src/app.js", "guardWriteAllowed"),
        ("web/dashboard/src/app.js", "cortexdb-dashboard-read-only"),
    ],
    "operational_status": [
        ("web/dashboard/src/index.html", "id=\"status-report\""),
        ("web/dashboard/src/index.html", "backup/restore posture"),
        ("web/dashboard/src/app.js", "dashboard_status.v1"),
        ("web/dashboard/src/app.js", "/v1/compatibility"),
        ("web/dashboard/src/app.js", "summarizeCompatibilityResult"),
        ("web/dashboard/src/app.js", "backup_posture"),
        ("web/dashboard/src/app.js", "backup_restore_view"),
        ("web/dashboard/src/app.js", "dashboard_backup_restore.v1"),
        ("web/dashboard/src/app.js", "incident_view"),
        ("web/dashboard/src/app.js", "dashboard_incident_view.v1"),
        ("web/dashboard/src/app.js", "actor_queue_depth"),
        ("web/dashboard/src/app.js", "actor_queue_capacity"),
        ("web/dashboard/src/app.js", "backup_latest_age_seconds"),
        ("web/dashboard/src/app.js", "rpo_budget_seconds"),
        ("web/dashboard/src/app.js", "rto_evidence_gate"),
        ("web/dashboard/src/app.js", "last_request_error"),
        ("web/dashboard/src/app.js", "make backup-restore-production-pack-check"),
        ("web/dashboard/src/app.js", "make backup-offsite-check"),
        ("web/dashboard/src/reporting_operations_status.js", "renderOperationalStatus"),
        ("web/dashboard/src/reporting_operations_status.js", "Version compatibility"),
        ("web/dashboard/src/reporting_operations_status.js", "API / SDK / storage / migration"),
        ("web/dashboard/src/reporting_operations_status.js", "Backup posture"),
        ("web/dashboard/src/reporting_operations_status.js", "Restore drill"),
        ("web/dashboard/src/reporting_operations_status.js", "Offsite status"),
        ("web/dashboard/src/reporting_operations_status.js", "RPO/RTO"),
        ("web/dashboard/src/reporting_operations_status.js", "Incident view"),
        ("web/dashboard/src/reporting_operations_status.js", "Incident errors"),
        ("web/dashboard/src/reporting_operations_status.js", "Rate limits"),
        ("web/dashboard/src/reporting_operations_status.js", "Actor busy"),
        ("web/dashboard/src/reporting_operations_status.js", "Storage warnings"),
        ("web/dashboard/src/reporting_operations_status.js", "Backup failures"),
        ("web/dashboard/src/reporting_operations_status.js", "Actor queue"),
        ("web/dashboard/src/reporting_operations_status.js", "Latest backup"),
        ("web/dashboard/src/reporting_operations_status.js", "Last error"),
        ("docs/archive/DASHBOARD_UI.md", "Operational Status View"),
        ("docs/archive/DASHBOARD_UI.md", "Backup/Restore View"),
        ("docs/archive/DASHBOARD_UI.md", "Incident View"),
    ],
    "incident_timeline": [
        ("web/dashboard/src/app.js", "incident_timeline"),
        ("web/dashboard/src/app.js", "buildIncidentTimeline"),
        ("web/dashboard/src/app.js", "audit_event"),
        ("web/dashboard/src/app.js", "rate_limit_event"),
        ("web/dashboard/src/app.js", "storage_event"),
        ("web/dashboard/src/app.js", "backup_event"),
        ("web/dashboard/src/reporting_operations_status.js", "renderIncidentEvent"),
        ("web/dashboard/src/reporting_operations_status.js", "Incident timeline"),
        ("web/dashboard/src/reporting_operations_status.js", "audit / rate / storage / backup"),
        ("docs/archive/DASHBOARD_UI.md", "Incident Timeline"),
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
        ("docs/archive/DASHBOARD_UI.md", "Single-node SLO Dashboard"),
        ("docs/archive/SINGLE_NODE_SLO.md", "dashboard_slo.v1"),
    ],
    "audit_readiness": [
        ("web/dashboard/src/index.html", "id=\"audit-report\""),
        ("web/dashboard/src/index.html", "id=\"audit-filter-category\""),
        ("web/dashboard/src/index.html", "id=\"audit-filter-severity\""),
        ("web/dashboard/src/index.html", "data-action=\"audit-readiness\""),
        ("web/dashboard/src/reporting_audit.js", "dashboard_audit_viewer.v2"),
        ("web/dashboard/src/reporting_audit.js", "renderAuditReadiness"),
        ("web/dashboard/src/reporting_audit.js", "hash_chain_verification"),
        ("web/dashboard/src/reporting_audit.js", "redaction_status"),
        ("web/dashboard/src/reporting_audit.js", "filtered_events"),
        ("docs/archive/DASHBOARD_UI.md", "Audit Viewer v2"),
    ],
    "permissions_view": [
        ("web/dashboard/src/index.html", "href=\"/dashboard/permissions\""),
        ("web/dashboard/src/dashboard_manifest.json", "\"permissions\""),
        ("web/dashboard/src/reporting_operations_permissions.js", "renderPermissionsView"),
        ("web/dashboard/src/app.js", "selected_scopes"),
        ("web/dashboard/src/app.js", "server_token_policy"),
        ("web/dashboard/src/app.js", "anonymous_synthetic_view"),
        ("web/dashboard/src/app.js", "dashboard_role_ui.v1"),
        ("web/dashboard/src/app.js", "roleUiState"),
        ("web/dashboard/src/app.js", "refreshDangerousOperationVisibility"),
        ("web/dashboard/src/index.html", "id=\"role-ui-report\""),
        ("web/dashboard/src/index.html", "data-dangerous=\"true\""),
        ("web/dashboard/src/app.js", "denials"),
        ("web/dashboard/src/reporting_operations_permissions.js", "Permissions explorer"),
        ("web/dashboard/src/reporting_operations_permissions.js", "Token / role / scope / AgentView"),
        ("web/dashboard/src/reporting_operations_permissions.js", "Role-based UI"),
        ("web/dashboard/src/reporting_operations_permissions.js", "Dangerous visible"),
        ("web/dashboard/src/reporting_operations_permissions.js", "Scope probes"),
        ("web/dashboard/src/reporting_operations_permissions.js", "AgentView policy"),
        ("web/dashboard/src/reporting_operations_permissions.js", "Denials"),
        ("docs/archive/DASHBOARD_UI.md", "Permissions Explorer"),
        ("docs/archive/DASHBOARD_UI.md", "Role-based Dashboard UI"),
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
        ("docs/archive/DASHBOARD_UI.md", "ContextPack Explorer"),
    ],
    "verification_explorer": [
        ("web/dashboard/src/index.html", "id=\"verify-report\""),
        ("web/dashboard/src/reporting_retrieval.js", "Mixed evidence"),
        ("web/dashboard/src/reporting_retrieval.js", "Contradicting evidence"),
        ("web/dashboard/src/reporting_retrieval.js", "Numeric conflict explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "Guard explorer"),
        ("web/dashboard/src/reporting_retrieval.js", "numeric_conflicts"),
        ("web/dashboard/src/reporting_retrieval.js", "source_trust_category"),
        ("docs/archive/DASHBOARD_UI.md", "Verification Explorer"),
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
        ("docs/archive/DASHBOARD_UI.md", "Ingestion Job Dashboard"),
    ],
    "release_artifacts": [
        ("e2e/dashboard_screenshots.mjs", "permissions"),
        ("docs/archive/DASHBOARD_UI.md", "dashboard-screenshots"),
        ("docs/archive/DASHBOARD_PRODUCT_UI_EVIDENCE.md", "make dashboard-product-check"),
    ],
}


def read(path: Path) -> str:
    try:
        if path == Path("web/dashboard/src/app.js"):
            return read_app_sources()
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
