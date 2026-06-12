#!/usr/bin/env python3
"""Build deterministic evidence-slot plans for EnterpriseRAG-Bench answers."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from question_decomposition import precise_anchors, split_subquestions


SCHEMA_VERSION = "cortexdb.enterprise_rag_bench.evidence_slot_plan.v1"


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def has_any(text: str, terms: tuple[str, ...]) -> bool:
    return any(term in text for term in terms)


def add_slot(
    slots: list[dict[str, Any]],
    seen: set[str],
    *,
    kind: str,
    label: str,
    instruction: str,
    required: bool = True,
    source_hint: str = "retrieved documents",
) -> None:
    key = f"{kind}:{label.lower()}"
    if key in seen:
        return
    seen.add(key)
    slots.append(
        {
            "id": f"s{len(slots) + 1:02d}",
            "kind": kind,
            "label": label,
            "required": required,
            "instruction": instruction,
            "source_hint": source_hint,
        }
    )


def normalize_question_type(question_type: Any) -> str:
    return str(question_type or "unknown").lower().strip()


def add_question_word_slots(slots: list[dict[str, Any]], seen: set[str], question: str) -> None:
    lower = question.lower()
    if lower.startswith("who ") or " who " in lower:
        add_slot(
            slots,
            seen,
            kind="person_or_role",
            label="person, owner, approver, reviewer, or role",
            instruction="Extract the exact person, team, owner, approver, reviewer, or role requested.",
        )
    if lower.startswith("when ") or " when " in lower:
        add_slot(
            slots,
            seen,
            kind="time",
            label="date, time, schedule, or window",
            instruction="Extract the exact date, time, schedule, window, or timezone requested.",
        )
    if lower.startswith("where ") or " where " in lower:
        add_slot(
            slots,
            seen,
            kind="location",
            label="location, region, cluster, route, or environment",
            instruction="Extract the exact location, region, cluster, route, or environment.",
        )
    if lower.startswith("why ") or " why " in lower or has_any(lower, ("root cause", "cause", "caused")):
        add_slot(
            slots,
            seen,
            kind="cause",
            label="root cause, trigger, or reason",
            instruction="Extract the directly stated root cause, trigger, mechanism, or reason.",
        )
    if lower.startswith("how ") or " how " in lower:
        add_slot(
            slots,
            seen,
            kind="method_or_process",
            label="method, process, sequence, or operational steps",
            instruction="Extract the method, process, ordered steps, checks, or operational actions.",
        )


def add_keyword_slots(slots: list[dict[str, Any]], seen: set[str], question: str) -> None:
    lower = question.lower()
    keyword_rules = [
        (
            ("threshold", "limit", "gate", "pass rate", "cutoff", "budget"),
            "threshold",
            "threshold, limit, gate, cutoff, or budget",
            "Extract exact thresholds, limits, gates, budgets, units, and comparison direction.",
        ),
        (
            ("latency", "p95", "p99", "ms", "rtt", "sla", "slo"),
            "metric",
            "metric, latency, SLO, SLA, or target",
            "Extract exact metric names, values, units, target windows, and pass/fail criteria.",
        ),
        (
            ("path", "file", "filename", "header", "config", "key"),
            "literal",
            "literal path, file, header, config, or key",
            "Copy literal paths, filenames, headers, config keys, and punctuation exactly.",
        ),
        (
            ("status", "state", "blocker", "blocked", "mitigation", "rollback"),
            "status",
            "status, blocker, mitigation, or rollback",
            "Extract the exact state, blocker, mitigation, rollback, or verification status.",
        ),
        (
            ("price", "cost", "billing", "invoice", "credits", "cheapest", "lowest", "highest"),
            "comparison",
            "numeric comparison or cost",
            "Compare all visible numeric candidates and extract the selected value with units.",
        ),
        (
            ("all ", "every ", "list", "which", "what are"),
            "completeness",
            "all requested items",
            "Include every requested item that is supported by matching evidence, not only the first one.",
        ),
    ]
    for terms, kind, label, instruction in keyword_rules:
        if has_any(lower, terms):
            add_slot(slots, seen, kind=kind, label=label, instruction=instruction)


def add_type_slots(slots: list[dict[str, Any]], seen: set[str], question_type: str) -> None:
    if question_type == "project_related":
        add_slot(
            slots,
            seen,
            kind="project_identity",
            label="project, incident, ticket, account, or product identity",
            instruction="Identify the exact project, incident, ticket, account, product, or tenant.",
        )
        add_slot(
            slots,
            seen,
            kind="project_state",
            label="project status, owner, blocker, deadline, and action",
            instruction="Extract status, owner, blockers, deadlines, remediation, and next actions when present.",
            required=True,
        )
        add_slot(
            slots,
            seen,
            kind="project_chain",
            label="related tickets, PRs, docs, and linked artifacts",
            instruction="Collect linked artifacts (tickets, threads, pages, commits) and evidence that belongs to the same project chain.",
        )
        return
    if question_type == "conflicting_info":
        add_slot(
            slots,
            seen,
            kind="conflict_pair",
            label="conflicting values and sources",
            instruction="Extract both conflicting claims, their source context, and which one is newer or authoritative.",
        )
    elif question_type == "constrained":
        add_slot(
            slots,
            seen,
            kind="constraint",
            label="hard filters, scope, and qualification conditions",
            instruction="Apply every explicit filter before answering: source type, dates, project scope, status, owner, and scope limitations.",
            required=True,
        )
    elif question_type == "completeness":
        add_slot(
            slots,
            seen,
            kind="coverage_checklist",
            label="complete checklist of requested subparts",
            instruction="Cover each sub-requirement and name missing requested parts if evidence is absent.",
        )
    elif question_type == "high_level":
        add_slot(
            slots,
            seen,
            kind="synthesis_theme",
            label="themes, representative evidence, and caveats",
            instruction="Synthesize grounded themes from representative sources and avoid generic unsupported claims.",
        )
    elif question_type in {"info_not_found", "unavailable", "null_query"}:
        add_slot(
            slots,
            seen,
            kind="answerability",
            label="answerability evidence",
            instruction="Confirm whether any retrieved document directly supports the answer; otherwise abstain.",
        )
    elif question_type in {"miscellaneous"}:
        add_slot(
            slots,
            seen,
            kind="topic_summary",
            label="all requested themes and outcomes",
            instruction="Collect all distinct themes the question implies and summarize evidence for each theme with concrete details.",
        )
    elif question_type == "intra_document_reasoning":
        add_slot(
            slots,
            seen,
            kind="argument_chain",
            label="linked causes, dependencies, and evidence chain",
            instruction="Build a cause->effect->resolution chain from the same document set and keep links explicit.",
        )
        add_slot(
            slots,
            seen,
            kind="risk_chain",
            label="risk, condition, and scope",
            instruction="Capture explicit risks, conditions, and scope statements tied to the same incident or request.",
        )
    elif question_type == "semantic":
        add_slot(
            slots,
            seen,
            kind="semantic_match",
            label="exact semantic match plus concrete anchors",
            instruction="Use meaning plus concrete anchors; avoid similarly worded but different scenarios.",
        )
        add_slot(
            slots,
            seen,
            kind="direct_answer",
            label="single best matched answer candidate",
            instruction="Select the strongest direct answer candidate and cite its exact factual details from evidence.",
        )
        add_slot(
            slots,
            seen,
            kind="semantic_coverage",
            label="entity/term aliases and variants",
            instruction="Prefer entities and aliases that exactly match question wording before fallback semantic matches.",
        )
    elif question_type in {"basic", "miscellaneous"}:
        add_slot(
            slots,
            seen,
            kind="basic_fact",
            label="directly asked fact(s)",
            instruction="Extract the direct fact(s) that answer the question and keep literal names/values.",
        )


def answer_policy(question_type: str, question: str) -> str:
    lower = question.lower()
    if question_type in {"info_not_found", "unavailable", "null_query"}:
        return "abstain_if_no_direct_support"
    if question_type in {"constrained", "miscellaneous"}:
        return "strict_constraint_first"
    if question_type == "completeness":
        return "fill_all_subparts_first"
    if question_type == "project_related":
        return "fill_project_chain_and_slots"
    if question_type == "high_level":
        return "synthesize_representative_coverage"
    if question_type == "semantic":
        return "fill_all_required_slots"
    if question_type == "conflicting_info":
        return "compare_conflicting_evidence"
    if question_type == "intra_document_reasoning":
        return "build_and_compare_evidence_chain"
    if has_any(lower, ("all ", "every ", "list")):
        return "fill_all_required_slots"
    return "fill_required_slots_compactly"


def build_evidence_plan(row: dict[str, Any]) -> dict[str, Any]:
    question = str(row.get("question") or "")
    question_type = normalize_question_type(row.get("question_type"))
    slots: list[dict[str, Any]] = []
    seen: set[str] = set()

    add_type_slots(slots, seen, question_type)
    add_question_word_slots(slots, seen, question)
    add_keyword_slots(slots, seen, question)
    if not slots:
        add_slot(
            slots,
            seen,
            kind="direct_answer",
            label="direct answer fact",
            instruction="Extract the direct fact that answers the question from matching evidence.",
        )

    anchors = precise_anchors(question)[:12]
    subquestions = split_subquestions(question)[:8]
    policy = answer_policy(question_type, question)
    return {
        "schema_version": SCHEMA_VERSION,
        "question_id": row.get("question_id"),
        "question_type": question_type,
        "question": question,
        "answer_policy": policy,
        "abstention_policy": (
            "Return exactly 'Insufficient information.' when no retrieved document supports any required slot."
        ),
        "anchors": anchors,
        "subquestions": subquestions,
        "slots": slots,
        "required_slot_count": sum(1 for slot in slots if slot["required"]),
    }


def format_evidence_plan_for_prompt(plan: dict[str, Any]) -> str:
    lines = [
        "Evidence slot plan:",
        f"- Answer policy: {plan.get('answer_policy')}",
        f"- Abstention: {plan.get('abstention_policy')}",
    ]
    anchors = [str(item) for item in plan.get("anchors", []) if str(item).strip()]
    if anchors:
        lines.append("- Exact anchors to verify: " + "; ".join(anchors[:10]))
    subquestions = [str(item) for item in plan.get("subquestions", []) if str(item).strip()]
    if subquestions:
        lines.append("- Required subquestions:")
        for item in subquestions[:6]:
            lines.append(f"  - {item}")
    lines.append("- Required evidence slots:")
    for slot in plan.get("slots", []):
        required = "required" if slot.get("required") else "optional"
        lines.append(
            "  - {id} [{required}] {label}: {instruction}".format(
                id=slot.get("id"),
                required=required,
                label=slot.get("label"),
                instruction=slot.get("instruction"),
            )
        )
    checklist = [item for item in plan.get("checklist", []) if isinstance(item, dict)]
    if checklist:
        coverage_pct = plan.get("coverage_pct")
        lines.append(f"- Completeness coverage from retrieved evidence: {coverage_pct}%")
        lines.append("- Covered checklist items and strongest evidence:")
        for item in checklist[:10]:
            status = "covered" if item.get("covered") else "uncovered"
            lines.append(f"  - {item.get('id')} [{status}] {item.get('text')}")
            for evidence in list(item.get("evidence", []))[:2]:
                evidence_text = " ".join(str(evidence.get("text") or "").split())
                if len(evidence_text) > 260:
                    evidence_text = evidence_text[:256].rstrip() + " ..."
                lines.append(
                    "    evidence: doc={doc} rank={rank} signals={signals}: {text}".format(
                        doc=evidence.get("doc_id"),
                        rank=evidence.get("doc_rank"),
                        signals=",".join(str(signal) for signal in evidence.get("signals", [])[:4]),
                        text=evidence_text,
                    )
                )
        uncovered = [str(item) for item in plan.get("uncovered_unit_ids", []) if str(item)]
        if uncovered:
            lines.append("- Uncovered checklist item IDs: " + ", ".join(uncovered[:12]))
        repair_policy = str(plan.get("repair_policy") or "").strip()
        if repair_policy:
            lines.append("- Repair policy: " + repair_policy)
    project_card = plan.get("project_card")
    if isinstance(project_card, dict):
        anchors = [
            str(item)
            for item in project_card.get("identity_anchors", [])
            if str(item).strip()
        ]
        if anchors:
            lines.append("- Project/card anchors: " + "; ".join(anchors[:10]))
        policy = str(project_card.get("answer_policy") or "").strip()
        if policy:
            lines.append("- Project/card answer policy: " + policy)
        by_category = project_card.get("by_category")
        if isinstance(by_category, dict):
            lines.append("- Project/card evidence rows:")
            for category in (
                "identity",
                "status",
                "owner",
                "timeline",
                "risk",
                "action",
                "metric",
                "linked_artifact",
            ):
                rows = [row for row in by_category.get(category, []) if isinstance(row, dict)]
                if not rows:
                    continue
                lines.append(f"  - {category}:")
                for row in rows[:3]:
                    evidence_text = " ".join(str(row.get("text") or "").split())
                    if len(evidence_text) > 280:
                        evidence_text = evidence_text[:276].rstrip() + " ..."
                    lines.append(
                        "    source={doc} rank={rank} line={line}: {text}".format(
                            doc=row.get("doc_id"),
                            rank=row.get("doc_rank"),
                            line=row.get("line"),
                            text=evidence_text,
                        )
                    )
        missing = [str(item) for item in project_card.get("missing_categories", []) if str(item)]
        if missing:
            lines.append("- Project/card missing categories: " + ", ".join(missing[:8]))
    conflict_resolution = plan.get("conflict_resolution")
    if isinstance(conflict_resolution, dict):
        anchors = [
            str(item)
            for item in conflict_resolution.get("anchors", [])
            if str(item).strip()
        ]
        if anchors:
            lines.append("- Conflict-resolution anchors: " + "; ".join(anchors[:10]))
        policy = str(conflict_resolution.get("answer_policy") or "").strip()
        if policy:
            lines.append("- Conflict-resolution answer policy: " + policy)
        by_kind = conflict_resolution.get("by_kind")
        if isinstance(by_kind, dict):
            lines.append("- Conflict-resolution claims:")
            for kind in ("current", "conflict", "previous", "candidate"):
                rows = [row for row in by_kind.get(kind, []) if isinstance(row, dict)]
                if not rows:
                    continue
                lines.append(f"  - {kind}:")
                for row in rows[:4]:
                    text = " ".join(str(row.get("text") or "").split())
                    if len(text) > 280:
                        text = text[:276].rstrip() + " ..."
                    markers = ",".join(str(marker) for marker in row.get("markers", [])[:5])
                    lines.append(
                        "    source={doc} rank={rank} date={date} markers={markers}: {text}".format(
                            doc=row.get("doc_id"),
                            rank=row.get("doc_rank"),
                            date=row.get("date") or "unknown",
                            markers=markers,
                            text=text,
                        )
                    )
    lines.append(
        "Fill these slots internally before writing the final answer; if a requested checklist item is uncovered, do not invent it."
    )
    return "\n".join(lines)
