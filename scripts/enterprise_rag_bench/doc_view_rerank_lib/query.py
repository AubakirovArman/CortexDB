from __future__ import annotations

from collections import Counter
from typing import Any

from question_decomposition import evidence_units, precise_anchors, tokens

from .constants import QUERY_EXPANSIONS

def query_terms(question: dict[str, Any]) -> list[str]:
    text = str(question.get("question", ""))
    values = tokens(text)
    for anchor in precise_anchors(text):
        values.extend(tokens(anchor))
    for token in list(values):
        values.extend(QUERY_EXPANSIONS.get(token, []))
    return sorted(set(values), key=lambda item: (-len(item), item))


def query_context(question: dict[str, Any]) -> dict[str, Any]:
    question_text = str(question.get("question", ""))
    terms = query_terms(question)
    return {
        "question_id": str(question.get("question_id", "")),
        "question_text": question_text,
        "terms": terms,
        "counts": Counter(terms),
        "source_types": {str(item) for item in question.get("source_types", []) if str(item)},
        "anchors": precise_anchors(question_text),
        "units": evidence_units(question_text),
    }
