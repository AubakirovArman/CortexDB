"""Span-plus-fallback ContextPack assembly for EnterpriseRAG-Bench."""

from __future__ import annotations

from context_windows import query_tokens, question_aware_snippet
from evidence_spans import EvidenceSpan, select_evidence_spans


def _coverage_ratio(spans: list[EvidenceSpan], question: str) -> float:
    question_terms = query_tokens(question)
    if not question_terms:
        return 0.0
    covered: set[str] = set()
    for span in spans:
        covered |= query_tokens(span.text) & question_terms
    return len(covered) / len(question_terms)


def evidence_span_plus_fallback_context(
    content: str,
    title: str,
    question: str,
    max_chars: int,
) -> str:
    """Return answerable spans plus source windows for completeness.

    Spans provide compact evidence units. Fallback windows preserve surrounding
    source context so generated answers do not lose caveats, ordering, or list
    completeness when span extraction is not exhaustive.
    """

    if max_chars <= 0:
        return ""

    spans = select_evidence_spans(content, question, max_spans=4, max_chars_per_span=900)
    if not spans:
        return question_aware_snippet(content, question, max_chars)

    coverage = _coverage_ratio(spans, question)
    span_budget = int(max_chars * (0.42 if coverage >= 0.55 else 0.32))
    span_budget = min(max(1400, span_budget), max(1400, max_chars - 1800))
    parts = [
        f"Materialized evidence spans for title: {title}",
        f"Coverage signal: query_term_coverage={coverage:.2f}",
    ]

    for index, span in enumerate(spans, 1):
        signals = ",".join(span.signals[:6]) or "score"
        block = (
            f"[Evidence span {index} | score={span.score:.2f} | signals={signals}]\n"
            f"{span.text}"
        )
        current = "\n\n".join(parts)
        remaining = span_budget - len(current) - len(block) - 2
        if remaining < 0:
            break
        parts.append(block)

    span_text = "\n\n".join(parts)
    fallback_budget = max(600, max_chars - len(span_text) - 80)
    fallback = question_aware_snippet(content, question, fallback_budget).strip()
    if fallback:
        return f"{span_text}\n\nFallback source windows:\n{fallback}"[:max_chars]
    return span_text[:max_chars]
