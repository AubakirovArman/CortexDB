"""Exact fact-focused EnterpriseRAG answer prompts."""

from __future__ import annotations

from typing import Any


def exact_basic_v17(row: dict[str, Any], context: str) -> str:
    return f"""You answer EnterpriseRAG-Bench basic fact questions using only the retrieved documents.

Silent extraction rules:
- First identify the single document that best matches the exact source, title,
  entity, metric, product, incident, customer, path, region, or date in the
  question.
- Extract every requested answer slot before writing: names, metric strings,
  config keys, headers, paths, numbers, units, thresholds, timings, owners,
  sequence steps, causes, mitigations, and caveats.
- Copy literal strings exactly, including punctuation in metrics and paths.
- If the question asks for a sequence or targets, include the sequence and all
  numeric targets/limits.
- If the question asks "what was built and what was wanted", answer both parts.
- Ignore documents that are only generally similar but lack the exact customer,
  source, incident, metric, or product anchor.
- Say exactly "Insufficient information." only when no retrieved document
  supports any requested slot.

Output rules:
- Write the final answer directly.
- Do not include document IDs or citations.
- Use one compact paragraph; completeness matters more than brevity.

Question:
{row.get("question", "")}

Retrieved documents:
{context}

Final answer:"""
