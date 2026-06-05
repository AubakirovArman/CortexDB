"""EnterpriseRAG-Bench answer prompt variants."""

from __future__ import annotations

from typing import Any


def evidence_selection_v5(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench questions using only the retrieved documents.

Before writing the answer, silently choose the exact evidence. Do not show this checklist.

Evidence selection rules:
- Match the exact entity, product, header, ticket, date, version, file path, region, metric, or numeric value named in the question.
- If several documents look similar, prefer the one with the most exact anchors from the question, not the highest-ranked generic match.
- If documents conflict, prefer incident-specific, current, dated, explicit, or directly quoted evidence over older or generic notes.
- For cheapest/lowest/highest questions, compare every visible numeric candidate and answer with the selected item plus its value.
- For path/file questions, copy the complete literal path exactly, including date suffixes and extensions.
- For list/role/process questions, include all required items from the supporting evidence; do not summarize away names or roles.
- If the context supports only part of the answer, answer that part. Do not append "Insufficient information." after a supported partial answer.
- Answer exactly "Insufficient information." only when none of the retrieved documents supports the requested answer.

Output rules:
- Write the final answer directly.
- Do not say "Based on the retrieved documents".
- Do not include document IDs or citations.
- Keep the answer compact, usually 1-4 sentences.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def fact_focused_v2(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench questions using only the retrieved documents.

Rules:
- Write the final answer directly; do not say "Based on the retrieved documents".
- Include every concrete name, ID, date, number, path, limit, region, ticket, or version that answers the question.
- If the context contains partial evidence, answer the supported parts. Do not append "Insufficient information" after a partial answer.
- Answer exactly "Insufficient information." only when none of the retrieved documents supports the requested answer.
- Prefer current, updated, explicit, or incident-specific evidence over older or generic notes.
- Avoid citations, document IDs, markdown headings, and long explanations.
- Keep the answer compact, usually 1-4 sentences.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def type_aware_v9(row: dict[str, Any], context: str) -> str:
    question_type = str(row.get("question_type") or "")
    if question_type == "project_related":
        return project_related_v9(row, context)
    if question_type == "semantic":
        return semantic_v9(row, context)
    return evidence_selection_v5(row, context)


def type_aware_v13(row: dict[str, Any], context: str) -> str:
    question_type = str(row.get("question_type") or "")
    if question_type in {
        "project_related",
        "conflicting_info",
        "constrained",
        "completeness",
        "miscellaneous",
    }:
        return source_of_truth_v13(row, context)
    return type_aware_v9(row, context)


def type_aware_v15(row: dict[str, Any], context: str) -> str:
    question_type = str(row.get("question_type") or "")
    if question_type in {
        "basic",
        "conflicting_info",
        "constrained",
        "miscellaneous",
        "project_related",
        "semantic",
    }:
        return evidence_coverage_v15(row, context)
    return type_aware_v13(row, context)


def evidence_coverage_v15(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench questions using only the retrieved documents.

Evidence coverage rules:
- Scan all documents and every Evidence digest bullet before answering.
- Prefer the document whose digest/window contains exact anchors and all requested fields.
- If old notes conflict with updated/current/FAQ/requirements docs, use the newer source.
- For root-cause questions, include cause, trigger, mechanism, impacted system,
  and deployed mitigation with exact header/path/limit/version names.
- For default/config questions, include the named config keys and exact units.
- For review/list/procedure questions, include every role, name, step, threshold,
  timing window, metric, and evidence-capture requirement visible in the source.
- For "how many" questions, count distinct documents/transcripts in context that match.
- After drafting, silently add any missing fact from a matching digest bullet.

Output rules: write the final answer directly, without document IDs or citations.
Be compact but complete. Say exactly "Insufficient information." only when no
retrieved document supports the question.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def source_of_truth_v13(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench questions using only the retrieved documents.

The context may include "Evidence digest" bullets before the document windows.
Use those bullets as high-priority candidate facts, then verify against the
document text. Do not use facts that are only from a similar but different
incident, SKU, header, rollout, interview, policy, or project.

Source-of-truth selection rules:
- If there are conflicting old notes and updated/current/FAQ/requirements docs,
  prefer the updated/current/FAQ/requirements source.
- If the question asks "standardizing", "default", "procedure", "does it
  support", "root cause", or "what mitigation", choose the document that
  directly states that exact decision, procedure, support status, cause, or
  deployed mitigation.
- For headers, paths, IDs, cutoff values, region names, and reason codes, copy
  the literal string exactly from the best matching evidence.
- For multi-step procedures, include every required step, ordering rule,
  verification metric, timing window, and evidence-capture requirement visible
  in the selected evidence.
- For interviews or strategy questions, list every strategy component visible
  in the selected evidence; do not stop after the first component.
- If two documents use different names for a similar thing, answer with the name
  from the source that matches the question's scenario and latest decision.

Output rules:
- Write the final answer directly.
- Do not include document IDs or citations.
- Be compact but complete; include all concrete facts needed to avoid a partial
  answer.
- Say exactly "Insufficient information." only when no retrieved document
  supports the question.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def project_related_v9(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench project and incident questions using only the retrieved documents.

Silently build an evidence table before answering. Do not show the table.

Project answer rules:
- Identify the exact project, tenant, incident, account, ticket, PR, date, region, route, product, or policy named in the question.
- Use only documents that match those anchors. Ignore nearby documents about similar incidents, tenants, products, headers, regions, or older guidance.
- If the question asks what happened, include root cause, trigger, impacted system, and why it was not a different cause.
- If the question asks remediation, include the concrete fix, rollout state, rollback or guardrail, verification metrics, dashboard/SLO checks, and time window.
- If the question asks approvals or policy, include every named approver, role, threshold, exception, and audit trail requirement.
- If there are multiple required steps, list all steps in the same order as the evidence. Do not compress away names, thresholds, dates, request IDs, ticket IDs, or paths.
- If some gold-like facts are missing from context, answer the supported facts and explicitly say which requested part is not available.
- Answer exactly "Insufficient information." only when no retrieved document supports the requested answer.

Output rules:
- Write the final answer directly.
- Use one compact paragraph or short semicolon-separated clauses.
- Do not include document IDs or citations.
- Do not say "Based on the retrieved documents".

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def semantic_v9(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench semantic questions using only the retrieved documents.

Silently decompose the question into required facts before answering. Do not show the decomposition.

Semantic answer rules:
- Select evidence by exact meaning plus concrete anchors: names, dates, files, paths, metrics, prices, limits, regions, versions, owners, and status labels.
- For "which", "what", "why", "how", "cheapest", "lowest", "highest", or comparison questions, inspect every visible candidate value before choosing.
- If one document gives a broad concept and another gives a concrete value, combine them only when they refer to the same entity or scenario.
- Prefer current, explicit, incident-specific, or spec-like evidence over generic notes.
- Avoid substituting a semantically similar but different approach, team, metric, header, store, rollout, or cost value.
- Include all concrete facts needed to make the answer complete, even if that takes more than 4 sentences.
- If the context only partially supports the answer, give the supported part and name the missing part.
- Answer exactly "Insufficient information." only when no retrieved document supports the requested answer.

Output rules:
- Write the final answer directly.
- Do not include document IDs or citations.
- Do not say "Based on the retrieved documents".
- Keep wording dense and factual.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def evidence_audit_v11(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench questions using only the retrieved documents.

The context may contain an "Evidence digest" before each document window. Treat
the digest as a shortlist of exact candidate facts, but verify it against the
document text before answering.

Silent audit checklist:
1. Break the question into every requested fact: entity, cause, exception,
   schedule, threshold, header, cutoff, default behavior, comparison, or caveat.
2. For tables or numeric comparisons, inspect every visible row and choose the
   row with the exact workload/entity named in the question.
3. Prefer documents whose title or summary matches the exact question anchors.
   Do not mix a generic rollout/runbook with a different incident, policy, SKU,
   header, region, or workload.
4. When documents conflict, prefer updated/current/FAQ/incident-specific
   evidence over older notes, and mention the superseded note only if relevant.
5. Include all concrete answer facts: names, IDs, dates, percentages, units,
   paths, headers, reason codes, limits, regions, and rollback/verification
   conditions.
6. Do not add approvers, PR IDs, requirements, or operational details unless
   they directly answer the question.
7. If the context supports only part of the answer, answer the supported part
   and name the missing requested part. Say exactly "Insufficient information."
   only when no retrieved document supports the question.

Output rules:
- Write the final answer directly.
- Do not show the audit checklist.
- Do not include document IDs or citations.
- Use a compact but complete answer; completeness is more important than being
  under four sentences.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""


def baseline(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench questions using only the retrieved documents.

Rules:
- If the documents do not contain enough evidence, answer exactly: Insufficient information.
- Be concise but include all required facts.
- Do not invent facts outside the context.
- Do not include document IDs unless they are necessary to disambiguate evidence.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Answer:"""


def build_prompt(row: dict[str, Any], context: str, prompt_style: str) -> str:
    if prompt_style == "evidence-selection-v5":
        return evidence_selection_v5(row, context)
    if prompt_style == "type-aware-v9":
        return type_aware_v9(row, context)
    if prompt_style == "type-aware-v13":
        return type_aware_v13(row, context)
    if prompt_style == "type-aware-v15":
        return type_aware_v15(row, context)
    if prompt_style == "evidence-audit-v11":
        return evidence_audit_v11(row, context)
    if prompt_style == "fact-focused-v2":
        return fact_focused_v2(row, context)
    return baseline(row, context)
