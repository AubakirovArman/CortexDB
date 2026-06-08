"""Coverage-oriented EnterpriseRAG answer prompts."""

from __future__ import annotations

from typing import Any


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
