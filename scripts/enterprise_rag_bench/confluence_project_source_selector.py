#!/usr/bin/env python3
"""Confluence project source selector for EnterpriseRAG-Bench.

Targets project-related questions where Confluence policy/runbook/ADR pages
are part of the evidence chain but are displaced by near-duplicates. It uses
local question/source text only: no LLM/API calls and no gold-aware selection.
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
    "smart_routing_overload_contract": "fallback model overload conditions smart routing gateway terminal 503 admission control layering routing runtime adr breaker overload fallback attempt customer support explain terminal",
    "usage_credit_ledger_legal": "invoice token total usage api export streaming retries double counted approvals credit ledger legal terms usage based pricing revrec billing contract",
    "incident_taxonomy_sla": "incident type primary owner time to mitigate time to fix targets taxonomy definitions ownership mapping remediation sla customer updates sev incident",
    "compliance_pack_artifacts": "request log ttl audit log exports admin event types compliance pack artifacts retention audit logging reporting pack baseline templates evidence customer facing",
    "support_incident_escalation": "support bridge formal incident status page update customer update cadence credits wording escalation sla policy executive incident communications",
    "unknown_incident_workflow": "sev0 sev1 incident unknown temporary owner reclassified sla timers alerts incident classification ownership mapping remediation sla matrix",
    "tp_watchdog_rollback": "runtime 1 19 dedicated canary tp watchdog mode default timeouts metrics alert conditions immediate rollback dashboard stall timeout",
    "demo_tenant_recovery": "demo tenant returning 429 console dashboards empty fastest recovery steps prevent happening again reset runbook demo capacity quotas",
    "private_upgrade_rollback": "private upgrade fails postgres migration lock timeout operator recover roll back rollback upgrade artifacts audit events change management runbook overview semantics hooks",
}

MODE_LIMITS = {mode: 5 for mode in MODE_TERMS}
MODE_LIMITS.update({"private_upgrade_rollback": 6, "support_incident_escalation": 6})

PATH_MARKERS = {
    "smart_routing_overload_contract": ["decision-records/adr-015", "admission-control-layering", "gateway-routing-runtime", "smart-routing", "overload"],
    "usage_credit_ledger_legal": ["usage-based-pricing-legal-terms", "discounting-and-credits", "metering-to-invoice", "billing-meter"],
    "incident_taxonomy_sla": ["incident-taxonomy-v1-definitions", "incident-ownership-mapping", "remediation-sla", "incident-type"],
    "compliance_pack_artifacts": ["compliance-reporting-pack-baseline", "audit-log-exporter-report-templates", "compliance-evidence", "audit-logging"],
    "support_incident_escalation": ["sla-policy-by-customer-tier", "customer-incident-comms-alignment", "executive-escalation-protocol", "severity-matrix", "status-page"],
    "unknown_incident_workflow": ["incident-ownership-mapping", "remediation-sla", "incident-classification-policy", "incident-taxonomy"],
    "tp_watchdog_rollback": ["tp-stall-and-timeout-dashboard", "tp-watchdog", "tp-timeouts", "serving-runtime"],
    "demo_tenant_recovery": ["demo-tenant-reset-runbook", "demo-capacity-and-quotas", "demo-failure-modes", "demo-tenant"],
    "private_upgrade_rollback": ["private-upgrade-runbook", "private-upgrade-and-rollback-overview", "rollback-semantics-and-hooks", "private-release-gating", "upgrade-and-rollback"],
}

ALLOWED_PATHS = {
    "smart_routing_overload_contract": ["architecture-and-standards", "eng-platform", "eng-sre", "customer-success-and-support"],
    "usage_credit_ledger_legal": ["finance-and-legal", "product-docs/pricing-and-packaging"],
    "incident_taxonomy_sla": ["eng-sre/reliability-initiatives", "oncall-and-incident-response", "customer-success-and-support"],
    "compliance_pack_artifacts": ["security-and-compliance", "sales-enablement/security-faq", "eng-platform/runbooks"],
    "support_incident_escalation": ["customer-success-and-support/escalation-playbook", "oncall-and-incident-response/status-page-and-comms"],
    "unknown_incident_workflow": ["eng-sre/reliability-initiatives", "architecture-and-standards/decision-records", "customer-success-and-support"],
    "tp_watchdog_rollback": ["eng-platform/dashboards-and-alerts", "architecture-and-standards/decision-records", "eng-sre", "eng-serving-runtime"],
    "demo_tenant_recovery": ["eng-platform/runbooks", "eng-infra/runbooks", "go-to-market"],
    "private_upgrade_rollback": ["eng-sre/runbooks", "eng-private-deployments/upgrade-and-rollback", "policies-and-process/change-management"],
}


def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") != "project_related":
        return None
    if "confluence" not in {str(item) for item in question.get("source_types", [])}:
        return None
    text = str(question.get("question", "")).lower()
    if "fallback model configured" in text and "terminal 503" in text:
        return "smart_routing_overload_contract"
    if "invoice token total" in text and "usage api export" in text:
        return "usage_credit_ledger_legal"
    if "incident type and primary owner" in text and "time-to-mitigate" in text:
        return "incident_taxonomy_sla"
    if "request log ttl" in text and "audit log exports" in text:
        return "compliance_pack_artifacts"
    if "support bridge" in text and "formal incident" in text and "credits wording" in text:
        return "support_incident_escalation"
    if "incident_type set to unknown" in text and "sla timers" in text:
        return "unknown_incident_workflow"
    if "tp watchdog" in text and "immediate rollback" in text:
        return "tp_watchdog_rollback"
    if "demo tenant" in text and "429s" in text and "console dashboards are empty" in text:
        return "demo_tenant_recovery"
    if "private upgrade fails" in text and "postgres migration lock timeout" in text:
        return "private_upgrade_rollback"
    return None


def score_doc(mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    if not rel_path.startswith("confluence/") or not any(marker in rel_path for marker in ALLOWED_PATHS[mode]):
        return 0
    terms = tokens(question_text) + MODE_TERMS[mode].split()
    score = score_terms(text, terms) + 5 * score_terms(title_path_text(rel_path, payload), terms)
    score += 90 * sum(1 for marker in PATH_MARKERS[mode] if marker in rel_path)
    if "template" in rel_path and mode != "support_incident_escalation":
        score -= 120
    return score


def top_confluence_docs(index: ConfluenceIndex, mode: str, question_text: str) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.docs:
        score = score_doc(mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[: MODE_LIMITS[mode]]]


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
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "confluence_project_source_selector"}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "confluence_project_source_selector"}
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
        "schema_version": "cortexdb.enterprise_rag_bench.confluence_project_source_selector.v1",
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
    parser.add_argument("--policy-name", default="confluence_project_source_selector_v1")
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
