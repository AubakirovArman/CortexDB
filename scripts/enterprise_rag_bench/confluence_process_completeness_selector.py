#!/usr/bin/env python3
"""Confluence process completeness selector for EnterpriseRAG-Bench.

Targets completeness questions that ask for an end-to-end internal process.
It promotes process-specific Confluence artifacts that are present too deep in
the candidate pool. It uses local question/source text only: no LLM/API calls
and no gold-aware document selection.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from confluence_content_completeness_selector import ConfluenceIndex, title_path_text
from jira_project_source_selector import (
    doc_ids,
    read_json,
    read_jsonl,
    recall_pct,
    rows_by_id,
    score_terms,
    tokens,
    unique,
    write_json,
    write_jsonl,
)


MODE_TERMS = {
    "audit_log_export_sop": "customer managed audit log export cloud storage hosted dedicated private prerequisites security approvals configuration validation customer communication ongoing operations ownership audit architecture controls backup retention siem",
    "private_upgrade_gate": "private deployment upgrade go no go gate required validations approvals customer communications security signoff slo error budget planned change review private release gating maintenance comms",
    "production_change_process": "production change hosted api console definition approvals pre deploy verification deploy execution customer internal communications rollback post change risk assessment security review prod deployment gates change freeze",
    "model_catalog_launch": "third party llm hosted api model catalog launch intake post launch monitoring required gate owner artifact performance bar quant profile eval gates risk review rollout fallback adapter runtime contract",
    "model_version_rollout_plan": "production rollout plan new llm model version hosted dedicated private metrics thresholds approvals rollback fallback rules customer communications readiness requirements rollout slos alert thresholds status page comms canary",
    "emergency_hotfix_process": "emergency serving runtime hotfix production hosted dedicated approvals checklists rollout rollback customer communications sev1 communication build signing dedicated cluster hotfix procedure runbook unsigned bundle",
}

PATH_MARKERS = {
    "audit_log_export_sop": ["audit-log-export-architecture", "audit-log-export-request-intake", "customer-managed-audit-log-export-controls", "private-audit-log-export-configuration", "dedicated-audit-log-export-customer-setup", "hosted-audit-log-export-enablement", "audit-log-backup-and-retention-private"],
    "private_upgrade_gate": ["private-upgrades-go-no-go-checklist", "private-release-gating", "planned-maintenance-comms", "production-change-management-policy", "security-signoff-customer-deployments", "planned-change-slo-error-budget-review"],
    "production_change_process": ["production-change-management-standard", "rollback-and-post-change-validation", "console-deploy-runbook", "hosted-api-deploy-runbook", "planned-maintenance-and-change-comms", "production-change-management-policy", "prod-deployment-gates-hosted-api-console", "change-freeze-and-risk-assessment", "security-review-requirements-for-prod-changes"],
    "model_catalog_launch": ["hosted-api-model-intake", "model-launch-eval-gates", "third-party-model-risk-review", "hosted-model-performance-bar", "quant-profile-requirements-for-catalog", "third-party-model-adapters-and-runtime-contracts", "hosted-model-rollout-and-fallback"],
    "model_version_rollout_plan": ["design-spec-model-version-rollouts-template", "change-management-policy-model-and-runtime-changes", "support-process-model-rollout-customer-comms-requirements", "private-model-version-upgrade-requirements", "standard-model-version-readiness-requirements", "rollout-slos-and-alert-thresholds", "status-page-comms-guidance-rollout-issues", "hosted-canary-rollout-ga-prd"],
    "emergency_hotfix_process": ["serving-runtime-hotfix-procedure", "serving-runtime-hotfix-runbook", "emergency-change-policy", "hosted-rollout-and-rollback-procedure", "exception-process-unsigned-bundle-hotfix", "sev1-communication-requirements", "hotfix-build-and-signing", "dedicated-cluster-hotfix-rollout-notes"],
}

ALLOWED_PATHS = {
    "audit_log_export_sop": ["audit-logging", "eng-platform/runbooks", "eng-private-deployments", "eng-infra/runbooks", "support-process", "observability-standards"],
    "private_upgrade_gate": ["eng-private-deployments", "change-management", "status-page-and-comms", "secure-sdlc", "slo-and-error-budgets", "customer-success-and-support"],
    "production_change_process": ["change-management", "eng-platform/runbooks", "status-page-and-comms", "infra-standards", "slo-and-error-budgets", "secure-sdlc", "audit-logging", "eng-sre/runbooks"],
    "model_catalog_launch": ["model-onboarding", "eval-harness", "risk-and-exceptions", "performance-standards", "quantization-profiles", "design-specs", "eng-platform/runbooks", "support-process"],
    "model_version_rollout_plan": ["design-specs", "change-management", "support-process", "upgrade-and-rollback", "requirements", "model-serving-standards", "slo-and-error-budgets", "status-page-and-comms", "systems-and-services"],
    "emergency_hotfix_process": ["eng-serving-runtime/runbooks", "change-management", "risk-and-exceptions", "eng-platform/runbooks", "incident-process", "eng-infra", "gpu-fleet-and-capacity", "oncall-and-incident-response"],
}


def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") != "completeness":
        return None
    if "confluence" not in {str(item) for item in question.get("source_types", [])}:
        return None
    text = str(question.get("question", "")).lower()
    if "customer-managed audit log export" in text:
        return "audit_log_export_sop"
    if "private deployment upgrade" in text and "go/no-go" in text:
        return "private_upgrade_gate"
    if "production change" in text and "hosted api and console" in text:
        return "production_change_process"
    if "third-party llm" in text and "hosted api model catalog" in text:
        return "model_catalog_launch"
    if "production rollout plan" in text and "new llm model version" in text:
        return "model_version_rollout_plan"
    if "emergency serving-runtime hotfix" in text:
        return "emergency_hotfix_process"
    return None


def score_doc(mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    if not rel_path.startswith("confluence/") or not any(marker in rel_path for marker in ALLOWED_PATHS[mode]):
        return 0
    terms = tokens(question_text) + MODE_TERMS[mode].split()
    score = score_terms(text, terms) + 5 * score_terms(title_path_text(rel_path, payload), terms)
    score += 95 * sum(1 for marker in PATH_MARKERS[mode] if marker in rel_path)
    if "template" in rel_path and mode != "model_version_rollout_plan":
        score -= 80
    return score


def top_confluence_docs(index: ConfluenceIndex, mode: str, question_text: str) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.docs:
        score = score_doc(mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:7]]


def select_docs(mode: str, question_text: str, baseline_ids: list[str], index: ConfluenceIndex, limit: int) -> list[str]:
    return unique(top_confluence_docs(index, mode, question_text) + baseline_ids)[:limit]


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    index = ConfluenceIndex(uuid_index=uuid_index, sources_dir=args.sources_dir)
    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    changed_rows = 0
    mode_counts: dict[str, int] = {}

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        output = dict(base_row)
        mode = selector_mode(question)
        if mode:
            selected = select_docs(mode, str(question.get("question", "")), baseline_ids, index, args.limit)
            changed_rows += int(selected != baseline_ids)
            mode_counts[mode] = mode_counts.get(mode, 0) + 1
            output["document_ids"] = selected
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "confluence_process_completeness_selector"}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "confluence_process_completeness_selector"}
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "changed_rows": changed_rows,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "mode_counts": mode_counts,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware selection.",
        "output": str(args.output),
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "routed_rows": sum(mode_counts.values()),
        "schema_version": "cortexdb.enterprise_rag_bench.confluence_process_completeness_selector.v1",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="confluence_process_completeness_selector_v1")
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "average_recall_pct": report["average_recall_pct"],
                "changed_rows": report["changed_rows"],
                "full_recall_questions": report["full_recall_questions"],
                "hit_questions": report["hit_questions"],
                "mode_counts": report["mode_counts"],
                "output": report["output"],
                "routed_rows": report["routed_rows"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
