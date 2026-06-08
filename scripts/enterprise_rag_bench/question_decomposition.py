"""Question decomposition helpers for EnterpriseRAG-Bench retrieval.

The benchmark often asks for a compact evidence set, not the single most
similar document. These helpers turn a question into small deterministic
evidence units that can be used for diagnostics and coverage-aware reranking.
"""

from __future__ import annotations

import re
from typing import Any


STOPWORDS = {
    "a",
    "about",
    "according",
    "after",
    "and",
    "are",
    "as",
    "at",
    "be",
    "before",
    "by",
    "does",
    "for",
    "from",
    "how",
    "if",
    "in",
    "including",
    "into",
    "is",
    "it",
    "of",
    "on",
    "or",
    "should",
    "that",
    "the",
    "their",
    "these",
    "this",
    "those",
    "to",
    "was",
    "were",
    "what",
    "when",
    "where",
    "which",
    "while",
    "who",
    "why",
    "with",
}


def normalize(text: str) -> str:
    return re.sub(r"[^a-z0-9_./:%-]+", " ", text.lower()).strip()


def tokens(text: str) -> list[str]:
    return [
        token
        for token in normalize(text).split()
        if len(token) > 1 and token not in STOPWORDS
    ]


def precise_anchors(question: str) -> list[str]:
    anchors: set[str] = set()
    patterns = [
        r"`([^`]+)`",
        r"\b[A-Z]{2,}[A-Z0-9_-]*\b",
        r"\b[A-Z][a-z]+(?:[A-Z][a-z0-9]+)+\b",
        r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+){1,4}\b",
        r"\b[a-zA-Z0-9_./:-]+\.[a-zA-Z0-9_./:-]+\b",
        r"\b[a-z]+-[a-z0-9-]+(?:-[a-z0-9]+)*\b",
        r"\b\d+(?:\.\d+)?(?:%|ms|s|mib|gib|gb|mb|hours?|minutes?|am|pm)?\b",
    ]
    for pattern in patterns:
        for match in re.findall(pattern, question):
            value = match if isinstance(match, str) else match[0]
            cleaned = normalize(value)
            if cleaned and cleaned not in STOPWORDS:
                anchors.add(cleaned)
    return sorted(anchors, key=lambda value: (-len(value), value))


def split_subquestions(question: str) -> list[str]:
    """Split one benchmark question into small search intents."""

    cleaned = re.sub(r"\s+", " ", question.strip())
    separators = [
        r"\band what\b",
        r"\band when\b",
        r"\band where\b",
        r"\band which\b",
        r"\band how\b",
        r"\band why\b",
        r"\bincluding\b",
        r"\bincluding the\b",
        r"\balong with\b",
        r"\bplus\b",
    ]
    pattern = "|".join(f"(?:{item})" for item in separators)
    raw_parts = re.split(pattern, cleaned, flags=re.IGNORECASE)

    parts: list[str] = []
    for part in raw_parts:
        stripped = part.strip(" ,;:.?")
        if len(tokens(stripped)) >= 2:
            parts.append(stripped)

    # Long list-style questions often hide multiple required slots after commas.
    for clause in re.split(r"[,;]", cleaned):
        stripped = clause.strip(" ,;:.?")
        if 2 <= len(tokens(stripped)) <= 8:
            parts.append(stripped)

    unique: list[str] = []
    seen: set[str] = set()
    for part in parts:
        key = normalize(part)
        if not key or key in seen:
            continue
        seen.add(key)
        unique.append(part)
    return unique[:8]


def expected_slots(question: str) -> list[str]:
    lower = question.lower()
    slots: list[str] = []
    if "when" in lower or "scheduled" in lower or "time window" in lower:
        slots.append("date time schedule window timezone")
    if any(term in lower for term in ("threshold", "limit", "pass rate", "gate", "size")):
        slots.append("threshold limit default pass rate gate size")
    if any(term in lower for term in ("latency", "p95", "p99", "ms", "rtt")):
        slots.append("latency p95 p99 ms rtt")
    if any(term in lower for term in ("cost", "price", "credits", "billing", "invoice")):
        slots.append("cost price credits billing invoice")
    if any(term in lower for term in ("cause", "root cause", "caused")):
        slots.append("root cause trigger reason")
    if any(term in lower for term in ("location", "region", "edge", "cluster")):
        slots.append("location region edge cluster")
    if any(term in lower for term in ("role", "owner", "review", "approver")):
        slots.append("role owner reviewer approver")
    return slots


def evidence_units(question: str) -> list[dict[str, Any]]:
    units: list[dict[str, Any]] = []
    seen: set[str] = set()

    def push(kind: str, text: str) -> None:
        unit_tokens = tokens(text)
        if not unit_tokens:
            return
        key = f"{kind}:{normalize(text)}"
        if key in seen:
            return
        seen.add(key)
        units.append(
            {
                "id": f"u{len(units) + 1:02d}",
                "kind": kind,
                "text": text,
                "tokens": unit_tokens,
            }
        )

    for anchor in precise_anchors(question):
        push("anchor", anchor)
    for slot in expected_slots(question):
        push("slot", slot)
    for part in split_subquestions(question):
        push("subquery", part)
    if not units:
        push("question", question)
    return units[:16]


def covered_unit_ids(
    units: list[dict[str, Any]],
    normalized_doc: str,
    doc_token_set: set[str],
) -> list[str]:
    covered: list[str] = []
    for unit in units:
        unit_id = str(unit["id"])
        kind = str(unit["kind"])
        text = normalize(str(unit["text"]))
        unit_tokens = [str(token) for token in unit.get("tokens", [])]
        if kind == "anchor":
            if text and text in normalized_doc:
                covered.append(unit_id)
            continue
        hits = sum(1 for token in set(unit_tokens) if token in doc_token_set)
        required = 1 if len(unit_tokens) <= 2 else max(2, int(len(set(unit_tokens)) * 0.45))
        if hits >= required:
            covered.append(unit_id)
    return covered
