#!/usr/bin/env python3
"""Oracle-free abstention classifier for EnterpriseRAG-Bench retrieval rows.

This module decides whether a zero-document retrieval row should be treated as
"no information" or routed to a company-scope / high-level answer path. It uses
only the question text and the retrieved document list — never question_type,
source_types, expected_doc_ids, gold_answer, or answer_facts.
"""

from __future__ import annotations

import re


HIGH_LEVEL_SIGNALS = {
    "mission",
    "vision",
    "strategy",
    "thesis",
    "positioning",
    "overview",
    "company",
    "business model",
    "departments",
    "revenue streams",
    "security posture",
    "differentiation",
    "north star",
}

EXACT_LITERAL_SIGNALS = {
    "what is the value",
    "what was the value",
    "what is the exact",
    "what is the name",
    "what are the values",
    "how many",
    "how much",
    "what number",
    "which number",
    "what threshold",
    "what limit",
    "what budget",
    "what size",
    "what date",
    "when did",
    "when was",
    "who is",
    "who was",
    "which file",
    "which path",
    "which ticket",
    "which pr",
    "which version",
}

# Tokens/questions that strongly suggest the answer is a concrete literal.
_LITERAL_ANCHOR_RE = re.compile(
    r"\b([A-Z][A-Z0-9]{1,12}-\d+|v?\d+\.\d+(?:\.\d+)?|20\d{2}-\d{1,2}-\d{1,2}|"
    r"/[A-Za-z0-9._~+\-/%]+|\d+(?:\.\d+)?\s*(?:%|percent|ms|sec|min|hour|day|"
    r"MiB|MB|GiB|GB|KiB|KB|USD|EUR|GBP))\b",
    re.IGNORECASE,
)


def _tokens(text: str) -> set[str]:
    return set(re.findall(r"[a-z]{2,}", text.lower()))


def is_high_level_question(question: str) -> bool:
    """Return True if the question looks like a company/strategy overview query.

    A high-level question has multiple company-scope signals and lacks concrete
    anchors such as ticket IDs, version numbers, paths, or explicit value asks.
    """

    lowered = question.lower()
    high_hits = sum(1 for signal in HIGH_LEVEL_SIGNALS if signal in lowered)
    if high_hits < 2:
        return False
    # If the question contains a concrete anchor it is probably not high-level.
    if _LITERAL_ANCHOR_RE.search(question):
        return False
    # Wh-words that often accompany high-level questions.
    wh_words = {"what", "how", "describe", "explain", "summarize"}
    if not (_tokens(question) & wh_words):
        return False
    return True


def requires_exact_literal(question: str) -> bool:
    """Return True if the question is asking for a specific concrete value/ID."""

    lowered = question.lower()
    if any(signal in lowered for signal in EXACT_LITERAL_SIGNALS):
        return True
    # Multiple concrete anchors strongly suggest a literal-value question.
    if len(_LITERAL_ANCHOR_RE.findall(question)) >= 2:
        return True
    return False


def abstain_decision(
    *,
    question: str,
    document_ids: list[str],
    dense_top1_score: float | None = None,
    abstain_similarity_threshold: float = 0.0,
) -> tuple[bool, str]:
    """Oracle-free abstain decision.

    Returns (should_abstain, reason).

    Rules:
    - Non-empty retrieval -> answer (no abstain).
    - Empty retrieval + high-level company question -> do NOT abstain; route to
      company-scope retrieval.
    - Empty retrieval + exact-literal question with no evidence -> abstain.
    - Empty retrieval + generic question and low dense similarity -> abstain.

    ``dense_top1_score`` and ``abstain_similarity_threshold`` are optional; when
    a dense retriever score is available, scores below the threshold support
    abstention for literal questions.
    """

    if document_ids:
        return False, "has_retrieved_documents"

    if is_high_level_question(question):
        return False, "high_level_company_scope"

    if requires_exact_literal(question):
        if dense_top1_score is not None and dense_top1_score < abstain_similarity_threshold:
            return True, "no_evidence_for_literal_low_similarity"
        return True, "no_evidence_for_literal"

    return True, "no_evidence"
