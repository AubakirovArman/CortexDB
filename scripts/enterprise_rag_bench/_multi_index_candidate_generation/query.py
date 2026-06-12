from __future__ import annotations

import re
from typing import Any

from question_decomposition import evidence_units, precise_anchors, tokens

PATH_STOPWORDS = {
    "and",
    "for",
    "the",
    "with",
    "json",
    "sources",
    "users",
    "shared",
    "drives",
    "team",
    "wiki",
}


QUERY_TYPE_ROUTE_PRESETS = {
    "basic": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
    "semantic": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
    "project_related": {
        "content_boost_limit": 80,
        "content_weight": 0.012,
        "path_weight": 1.15,
    },
    "completeness": {
        "content_boost_limit": 80,
        "content_weight": 0.012,
        "path_weight": 1.0,
    },
    "conflicting_info": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
    "constrained": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "content_existing_only": True,
        "path_weight": 1.0,
    },
    "intra_document_reasoning": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
}


def path_tokens(path: str, *, expand_ngrams: bool = False) -> list[str]:
    base_tokens = [
        token
        for token in tokens(path.replace(".json", " "))
        if token not in PATH_STOPWORDS and len(token) > 1
    ]
    if not expand_ngrams:
        return base_tokens
    expanded: set[str] = set(base_tokens)
    for token in base_tokens:
        pieces = [piece for piece in re.split(r"[-_/.:]+", token) if piece and piece not in PATH_STOPWORDS]
        expanded.update(piece for piece in pieces if len(piece) > 1)
        for width in (2, 3, 4):
            for index in range(0, max(0, len(pieces) - width + 1)):
                expanded.add("-".join(pieces[index : index + width]))
                expanded.add("_".join(pieces[index : index + width]))
    return sorted(expanded)


def enterprise_entities(question_text: str) -> list[str]:
    entities: set[str] = set()
    patterns = [
        r"`([^`]+)`",
        r"\b[A-Z]{2,}[A-Z0-9_-]*-\d+[A-Z0-9_-]*\b",
        r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z0-9]+){1,5}\b",
        r"\b[A-Z][a-z]+(?:[A-Z][a-z0-9]+)+\b",
        r"\b[a-z]+-[a-z0-9-]+(?:-[a-z0-9]+)*\b",
        r"\b[A-Za-z0-9_./:-]+/[A-Za-z0-9_./:-]+\b",
        r"\b\d+(?:\.\d+)?(?:%|ms|s|kb|mb|gb|mib|gib|hours?|minutes?)\b",
    ]
    for pattern in patterns:
        for match in re.findall(pattern, question_text):
            value = match if isinstance(match, str) else match[0]
            normalized = " ".join(tokens(value))
            if not normalized:
                continue
            entities.add(normalized)
            if " " in normalized:
                entities.add(normalized.replace(" ", "-"))
                entities.add(normalized.replace(" ", "_"))
            if "/" in normalized:
                entities.add(normalized.replace("/", "-"))
                entities.add(normalized.replace("/", "_"))
    return sorted(entities, key=lambda value: (-len(value), value))


def query_terms(question: dict[str, Any]) -> list[str]:
    question_text = str(question.get("question", ""))
    values = tokens(question_text)
    for anchor in precise_anchors(question_text):
        values.extend(tokens(anchor))
    for entity in enterprise_entities(question_text):
        values.extend(tokens(entity))
        values.append(entity)
    return sorted(set(values), key=lambda value: (-len(value), value))


def strong_uncapped_terms(question: dict[str, Any]) -> set[str]:
    strong: set[str] = set()
    question_text = str(question.get("question", ""))
    for anchor in precise_anchors(question_text):
        for token in tokens(anchor):
            if "-" in token or any(char.isdigit() for char in token):
                strong.add(token)
    for token in tokens(question_text):
        if "-" in token or any(char.isdigit() for char in token):
            strong.add(token)
        elif token in {"p50", "p90", "p95", "p99", "rpo", "rto"}:
            strong.add(token)
    return strong


def phrase_ngrams(values: list[str], *, widths: tuple[int, ...] = (2, 3, 4)) -> set[str]:
    phrases: set[str] = set()
    for width in widths:
        for index in range(0, max(0, len(values) - width + 1)):
            phrase_tokens = values[index : index + width]
            if not phrase_tokens:
                continue
            if not any(
                "-" in token or any(char.isdigit() for char in token) or len(token) >= 6
                for token in phrase_tokens
            ):
                continue
            phrases.add(" ".join(phrase_tokens))
    return phrases


def query_phrases(question: dict[str, Any]) -> list[str]:
    question_text = str(question.get("question", ""))
    phrases = phrase_ngrams(tokens(question_text))
    for unit in evidence_units(question_text):
        unit_tokens = [str(token) for token in unit.get("tokens", []) if str(token)]
        phrases.update(phrase_ngrams(unit_tokens))
    for anchor in precise_anchors(question_text):
        anchor_tokens = tokens(anchor)
        if anchor_tokens and (
            len(anchor_tokens) > 1
            or "-" in anchor
            or any(char.isdigit() for char in anchor)
        ):
            phrases.add(" ".join(anchor_tokens))
    return sorted(phrases, key=lambda value: (-len(value), value))


def path_query_terms(question: dict[str, Any]) -> list[str]:
    question_text = str(question.get("question", ""))
    values: list[str] = []
    for entity in enterprise_entities(question_text):
        values.append(entity)
        values.extend(tokens(entity))
    for anchor in precise_anchors(question_text):
        if any(char in anchor for char in "-_/.:") or any(char.isdigit() for char in anchor):
            values.append(anchor)
            values.extend(tokens(anchor))
    if not values:
        values = [
            term
            for term in query_terms(question)
            if any(char in term for char in "-_/.:") or any(char.isdigit() for char in term)
        ]
    return sorted(set(values), key=lambda value: (-len(value), value))

