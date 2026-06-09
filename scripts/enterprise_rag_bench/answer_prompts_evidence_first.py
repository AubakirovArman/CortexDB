"""Evidence-first prompt variants for compact EnterpriseRAG contexts."""

from __future__ import annotations

from typing import Any


def evidence_first_v18(row: dict[str, Any], context: str) -> str:
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
