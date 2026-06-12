"""Self-consistency repair prompts for EnterpriseRAG answers."""

from __future__ import annotations

from typing import Any


def should_self_consistency_repair(guard_report: dict[str, Any]) -> bool:
    return int(guard_report.get("flagged_count", 0) or 0) > 0


def build_self_consistency_repair_prompt(
    *,
    question: str,
    context: str,
    draft_answer: str,
    guard_report: dict[str, Any],
) -> str:
    markers = [str(item) for item in guard_report.get("unsupported_markers", []) if str(item)]
    marker_text = "\n".join(f"- {marker}" for marker in markers[:30]) or "- none"
    return f"""You are repairing an EnterpriseRAG-Bench answer using only retrieved evidence.

The draft answer contains concrete values, dates, IDs, versions, paths, or numbers
that were not found verbatim in the retrieved evidence. Rewrite the answer so
that every concrete factual claim is supported by the retrieved documents.

Rules:
- Preserve supported facts from the draft.
- Remove or generalize unsupported concrete values.
- Do not invent replacement values.
- If a requested value is absent, say that the retrieved evidence does not state it.
- Answer exactly "Insufficient information." only when no useful part is supported.
- Do not cite document IDs.

Question:
{question}

Unsupported concrete markers detected in the draft:
{marker_text}

Draft answer:
{draft_answer}

Retrieved documents:
{context}

Repaired final answer:"""
