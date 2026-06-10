"""Evidence-first prompt variants for compact EnterpriseRAG contexts."""

from __future__ import annotations

from typing import Any


def _type_aware_evidence_instructions(question_type: str) -> str:
    if question_type == "project_related":
        return """Project mode:
- Build one chain of the same project/incident across matching tickets, PRs, docs, and threads.
- Favor exact project/tenant identifiers and owner/state/date anchors before generic guidance text.
- If conflicting notes exist, include both the current fact and a note on alternatives when relevant."""
    if question_type == "conflicting_info":
        return """Conflict mode:
- Extract both conflicting claims and compare by recency/source authority.
- Keep the conflict explicit in the answer and avoid choosing a side without evidence."""
    if question_type == "completeness":
        return """Completeness mode:
- Cover every requested subpart that has supporting evidence.
- If anything is missing, state what evidence is missing instead of inventing it."""
    if question_type == "constrained":
        return """Constrained mode:
- Apply every explicit filter (scope, source system, status, region, owner, date window) before selecting facts.
- Do not use evidence that violates the asked constraints."""
    if question_type == "semantic":
        return """Semantic mode:
- Use exact anchors from the question plus semantic match.
- If multiple matching values exist, return the one tied to the exact target entity and scope."""
    if question_type == "high_level":
        return """High-level mode:
- Return grounded representative evidence across sources.
- Prefer diversity across source types over repeating one narrow cluster."""
    if question_type == "info_not_found":
        return """Not-found mode:
- If no evidence supports the requested fact, answer exactly "Insufficient information."
- If partial evidence exists, return only supported parts and indicate missing parts."""
    if question_type == "intra_document_reasoning":
        return """Reasoning mode:
- Keep cause/effect/dependency chain explicit and only include directly supported links."""
    return ""


def evidence_first_v18(row: dict[str, Any], context: str) -> str:
    question_type = str(row.get("question_type") or "unknown").lower()
    instructions = _type_aware_evidence_instructions(question_type)
    extra = f"\n\n{instructions}" if instructions else ""

    return f"""You answer EnterpriseRAG-Bench questions using only the evidence in this prompt.

The prompt may include:
1. Evidence slot plan: the facts the question asks for.
2. Evidence table: exact candidate facts extracted from retrieved documents.
3. Retrieved documents: compact evidence digests and supporting snippets.

Evidence-first rules:
- Treat the Evidence table as high-priority source-grounded facts.
- Fill every evidence slot that is directly supported by the Evidence table or document snippets.
- Do not answer "Insufficient information." when any table row or snippet directly supports the requested fact.
- If a table row and a snippet conflict, prefer the row/snippet whose document title and text match the exact question anchors.
- For lists, roles, steps, transcript counts, thresholds, dates, paths, headers, regions, and IDs, include every concrete item visible in the evidence.
- For project questions, assemble the answer from all matching project evidence rows, not just the first document.
- For completeness questions, cover every requested subpart before optimizing for brevity.
- Say exactly "Insufficient information." only when no evidence row and no snippet supports the requested answer.
{extra}

Output rules:
- Write the final answer directly.
- Do not include document IDs or citations.
- Be compact but complete.
- Do not explain your reasoning or mention the evidence table.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""
