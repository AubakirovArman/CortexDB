"""Prompt and context formatting helpers for MultiHop-RAG QA."""

from __future__ import annotations

import re
from typing import Any


WORD_RE = re.compile(r"[A-Za-z0-9]+")
SENTENCE_RE = re.compile(r"(?<=[.!?])\s+")
DATE_RE = re.compile(
    r"\b(?:20\d{2}|19\d{2}|Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|"
    r"May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:tember)?|Oct(?:ober)?|"
    r"Nov(?:ember)?|Dec(?:ember)?)[A-Za-z0-9,:\- ]{0,32}",
    re.IGNORECASE,
)


def tokenize(value: str) -> set[str]:
    stop = {
        "the",
        "and",
        "for",
        "with",
        "from",
        "that",
        "this",
        "what",
        "which",
        "who",
        "about",
        "reported",
        "article",
        "according",
        "information",
        "another",
        "both",
    }
    return {
        word.lower()
        for word in WORD_RE.findall(value)
        if len(word) > 2 and word.lower() not in stop
    }


def payload_parts(payload: str) -> tuple[dict[str, str], str]:
    header, _, body = payload.partition("\n\n")
    metadata: dict[str, str] = {}
    for line in header.splitlines():
        key, sep, value = line.partition("=")
        if sep:
            metadata[key.strip()] = value.strip()
    return metadata, body.strip()


def best_snippet(query: str, payload: str, max_chars: int) -> str:
    metadata, body = payload_parts(payload)
    query_terms = tokenize(query)
    sentences = [sentence.strip() for sentence in SENTENCE_RE.split(body) if sentence.strip()]
    scored = []
    for index, sentence in enumerate(sentences):
        words = tokenize(sentence)
        score = len(query_terms & words)
        if score:
            scored.append((score, -index, sentence))
    selected = [sentence for _, _, sentence in sorted(scored, reverse=True)[:4]]
    if not selected:
        selected = sentences[:4]
    snippet = " ".join(selected)
    prefix = " | ".join(
        value
        for value in [metadata.get("title", ""), metadata.get("source", ""), metadata.get("published_at", "")]
        if value
    )
    text = f"{prefix}\n{snippet}" if prefix else snippet
    return text[:max_chars]


def date_mentions(value: str, limit: int = 6) -> list[str]:
    seen = set()
    dates = []
    for match in DATE_RE.findall(value):
        date = " ".join(match.strip(" ,.;").split())
        if date and date.lower() not in seen:
            seen.add(date.lower())
            dates.append(date)
        if len(dates) >= limit:
            break
    return dates


def temporal_snippet(query: str, payload: str, max_chars: int) -> str:
    metadata, body = payload_parts(payload)
    query_terms = tokenize(query)
    temporal_terms = {
        "after",
        "before",
        "subsequent",
        "later",
        "earlier",
        "change",
        "changed",
        "inconsistency",
        "inconsistent",
        "consistent",
        "remained",
        "reported",
        "published",
    }
    sentences = [sentence.strip() for sentence in SENTENCE_RE.split(body) if sentence.strip()]
    scored = []
    for index, sentence in enumerate(sentences):
        lowered = sentence.lower()
        words = tokenize(sentence)
        score = len(query_terms & words)
        score += sum(1 for term in temporal_terms if term in lowered)
        score += min(3, len(date_mentions(sentence)))
        if score:
            scored.append((score, -index, sentence))
    selected = [sentence for _, _, sentence in sorted(scored, reverse=True)[:5]]
    if not selected:
        selected = sentences[:5]
    title = metadata.get("title", "")
    source = metadata.get("source", "")
    published_at = metadata.get("published_at", "")
    dates = ", ".join(date_mentions(" ".join([title, published_at, body]), limit=8))
    fields = [
        f"title: {title}" if title else "",
        f"source: {source}" if source else "",
        f"published_at: {published_at}" if published_at else "",
        f"date_mentions: {dates}" if dates else "",
        "event_snippet: " + " ".join(selected),
    ]
    return "\n".join(field for field in fields if field)[:max_chars]


def build_prompt(row: dict[str, Any], top_k: int, max_chars_per_doc: int, prompt_style: str) -> str:
    contexts = []
    for item in row.get("retrieval_list", [])[:top_k]:
        text = str(item.get("text", ""))
        if prompt_style == "multihop-v3" and row.get("question_type") == "temporal_query":
            snippet = temporal_snippet(str(row.get("query", "")), text, max_chars_per_doc)
        else:
            snippet = best_snippet(str(row.get("query", "")), text, max_chars_per_doc)
        if snippet:
            contexts.append(f"[{len(contexts) + 1}]\n{snippet}")
    question_type = str(row.get("question_type", ""))
    if prompt_style in {"multihop-v2", "multihop-v3"}:
        return typed_prompt(row, question_type, contexts, prompt_style)
    return legacy_prompt(row, contexts)


def typed_prompt(row: dict[str, Any], question_type: str, contexts: list[str], prompt_style: str) -> str:
    type_instruction = {
        "comparison_query": (
            "This is a comparison question. If the context supports both sides, "
            "answer with Yes or No."
        ),
        "temporal_query": (
            "This is a temporal question. Compare the dates or event order in "
            "the context and answer with Yes or No."
        ),
        "null_query": (
            "This is a null-query check. Answer Insufficient Information unless "
            "the context directly supports the requested entity or fact."
        ),
        "inference_query": (
            "This is an inference question. Combine the relevant context snippets "
            "and answer with the shortest supported entity, date, number, or phrase."
        ),
    }.get(question_type, "Use only the provided context.")
    if prompt_style == "multihop-v3" and question_type == "temporal_query":
        type_instruction = (
            "This is a temporal yes/no question. Use published_at as the report date "
            "when the question asks about reports or articles. Compare the event order, "
            "change, consistency, or inconsistency across the relevant documents. "
            "When at least two relevant documents are present, answer exactly Yes or No. "
            "Use Insufficient Information only when the retrieved context does not include "
            "the named reports, entities, or events."
        )
    return "\n\n".join(
        [
            "Answer the question using only the provided context.",
            type_instruction,
            "Use exactly one short answer.",
            "For yes/no questions, answer exactly Yes or No.",
            "If the context is insufficient, answer exactly: Insufficient Information",
            "Do not explain your reasoning.",
            "",
            f"Question type: {question_type}",
            f"Question: {row.get('query', '')}",
            "",
            "Context:",
            "\n\n".join(contexts),
            "",
            "Answer:",
        ]
    )


def legacy_prompt(row: dict[str, Any], contexts: list[str]) -> str:
    return "\n\n".join(
        [
            "Answer the question using only the provided context.",
            "The answer should be a short entity, name, date, number, or phrase.",
            "If the context is insufficient, answer exactly: Insufficient Information",
            "Do not explain your reasoning.",
            "",
            f"Question: {row.get('query', '')}",
            "",
            "Context:",
            "\n\n".join(contexts),
            "",
            "Answer:",
        ]
    )


def build_temporal_abstention_retry_prompt(row: dict[str, Any], top_k: int, max_chars_per_doc: int) -> str:
    contexts = []
    for item in row.get("retrieval_list", [])[:top_k]:
        snippet = temporal_snippet(str(row.get("query", "")), str(item.get("text", "")), max_chars_per_doc)
        if snippet:
            contexts.append(f"[{len(contexts) + 1}]\n{snippet}")
    return "\n\n".join(
        [
            "Answer the temporal question using only the provided context.",
            "The first pass abstained, but this retry should choose the best supported temporal answer when context snippets are present.",
            "Do not answer Insufficient Information unless there are no context snippets.",
            "For yes/no wording, answer exactly Yes or No.",
            "For consistent/inconsistent wording, answer exactly Consistent or Inconsistent.",
            "For 'which source/entity/side' wording, answer the shortest source/entity/side name, or Both when both apply.",
            "Do not explain your reasoning.",
            "",
            f"Question: {row.get('query', '')}",
            "",
            "Context:",
            "\n\n".join(contexts),
            "",
            "Answer:",
        ]
    )


def build_comparison_retry_prompt(row: dict[str, Any], top_k: int, max_chars_per_doc: int) -> str:
    contexts = []
    for item in row.get("retrieval_list", [])[:top_k]:
        snippet = best_snippet(str(row.get("query", "")), str(item.get("text", "")), max_chars_per_doc)
        if snippet:
            contexts.append(f"[{len(contexts) + 1}]\n{snippet}")
    return "\n\n".join(
        [
            "Answer the comparison question using only the provided context.",
            "The first pass may have been too strict. Re-check each clause independently.",
            "For 'both' questions, answer Yes when both requested clauses are supported, even if they concern different sources or sectors.",
            "For 'different', 'contrast', or 'different strategy' questions, answer Yes when the context shows a real difference.",
            "For 'align', 'same', 'similar', or 'consistent' questions, answer Yes when the context is compatible on the requested point.",
            "Answer No only when a requested side is unsupported, contradicted, or the requested relation is not supported.",
            "Do not answer Insufficient Information when at least two relevant context snippets are present.",
            "Use exactly one short answer. For yes/no wording, answer exactly Yes or No.",
            "Do not explain your reasoning.",
            "",
            f"Question: {row.get('query', '')}",
            "",
            "Context:",
            "\n\n".join(contexts),
            "",
            "Answer:",
        ]
    )


def normalize_temporal_answer_for_question(question: str, answer: str) -> str:
    question_words = " ".join(WORD_RE.findall(question.lower()))
    answer_words = " ".join(WORD_RE.findall(answer.lower()))
    if "consistent or inconsistent" in question_words or "agreement or disagreement" in question_words:
        if answer_words == "yes":
            return "Consistent"
        if answer_words == "no":
            return "Inconsistent"
        return answer

    yes_no_starter = re.match(r"^(was|did|has|after|between|before|is)\b", question_words) is not None
    if not yes_no_starter:
        return answer

    asks_inconsistent = any(
        term in question_words
        for term in ["inconsistency", "inconsistent", "disagreement", "disagree", "discrepancy"]
    )
    asks_consistent = any(
        term in question_words
        for term in ["consistency", "consistent", "agreement", "agree", "align"]
    )
    asks_change = any(term in question_words for term in ["change", "changed", "different"])
    compatible_answers = {"consistent", "agreement", "agree", "aligned", "same", "similar"}
    incompatible_answers = {"inconsistent", "disagreement", "disagree", "different", "changed", "change"}
    if answer_words in compatible_answers:
        if asks_inconsistent or asks_change:
            return "No"
        if asks_consistent:
            return "Yes"
    if answer_words in incompatible_answers:
        if asks_inconsistent or asks_change:
            return "Yes"
        if asks_consistent:
            return "No"
    return answer
