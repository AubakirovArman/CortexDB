#!/usr/bin/env python3
"""Question-aware context windowing for EnterpriseRAG-Bench answer generation."""

from __future__ import annotations

import re
from dataclasses import dataclass


STOPWORDS = {
    "a",
    "an",
    "and",
    "are",
    "as",
    "at",
    "be",
    "by",
    "did",
    "do",
    "does",
    "for",
    "from",
    "how",
    "in",
    "is",
    "it",
    "of",
    "on",
    "or",
    "should",
    "that",
    "the",
    "their",
    "them",
    "they",
    "this",
    "to",
    "was",
    "were",
    "what",
    "when",
    "where",
    "which",
    "who",
    "why",
    "with",
}


@dataclass(frozen=True)
class ScoredWindow:
    score: float
    start: int
    end: int


def query_tokens(text: str) -> set[str]:
    return {
        token
        for token in re.findall(r"[a-zA-Z0-9_./:-]+", text.lower())
        if len(token) > 1 and token not in STOPWORDS
    }


def _window_ranges(text_len: int, window_chars: int) -> list[tuple[int, int]]:
    if text_len <= window_chars:
        return [(0, text_len)]
    stride = max(120, window_chars // 2)
    ranges: list[tuple[int, int]] = []
    start = 0
    while start < text_len:
        end = min(text_len, start + window_chars)
        ranges.append((start, end))
        if end == text_len:
            break
        start += stride
    return ranges


def _line_context_ranges(content: str, window_chars: int) -> list[tuple[int, int]]:
    lines = content.splitlines(keepends=True)
    offsets: list[int] = []
    offset = 0
    for line in lines:
        offsets.append(offset)
        offset += len(line)

    ranges: list[tuple[int, int]] = []
    for index, line in enumerate(lines):
        if not line.strip():
            continue
        start_index = max(0, index - 2)
        end_index = min(len(lines), index + 4)
        start = offsets[start_index]
        end = offsets[end_index] if end_index < len(lines) else len(content)
        while end - start < window_chars and end_index < len(lines):
            end_index += 1
            end = offsets[end_index] if end_index < len(lines) else len(content)
        if end - start > window_chars:
            end = min(len(content), start + window_chars)
        ranges.append((start, end))
    return ranges


def _score_window(window: str, tokens: set[str], question: str) -> float:
    lowered = window.lower()
    window_tokens = query_tokens(window)
    overlap = tokens & window_tokens
    score = float(len(overlap))
    for token in overlap:
        if any(char.isdigit() for char in token):
            score += 1.5
        if any(char in token for char in "/._:-"):
            score += 1.0
        if len(token) >= 8:
            score += 0.35
    for phrase in re.findall(r"[A-Za-z0-9_./:-]+(?:\\s+[A-Za-z0-9_./:-]+){1,3}", question):
        phrase = phrase.lower()
        if len(phrase) >= 10 and phrase in lowered:
            score += 2.0
    for marker, bonus in {
        "core concepts and definitions": 5.0,
        "unit primitives": 4.0,
        "root_cause": 5.0,
        "root cause": 4.0,
        "raw (annotated) numbers": 4.0,
        "$/1k": 3.0,
        "/confluence": 6.0,
        "pricing catalog": 4.0,
        "token definitions": 4.0,
        "required": 2.0,
        "mandatory": 2.0,
        "threshold": 2.0,
        "references": 1.5,
    }.items():
        if marker in lowered:
            score += bonus
    return score


def _overlaps(candidate: ScoredWindow, selected: list[ScoredWindow]) -> bool:
    for window in selected:
        overlap = max(0, min(candidate.end, window.end) - max(candidate.start, window.start))
        if overlap >= min(candidate.end - candidate.start, window.end - window.start) * 0.25:
            return True
    return False


def _select_diverse_windows(
    scored: list[ScoredWindow],
    content: str,
    tokens: set[str],
    limit: int,
) -> list[ScoredWindow]:
    selected: list[ScoredWindow] = []
    covered: set[str] = set()
    candidates = [window for window in scored if window.score > 0]
    while candidates and len(selected) < limit:
        best: ScoredWindow | None = None
        best_adjusted = -1.0
        for window in candidates:
            if _overlaps(window, selected):
                continue
            window_tokens = query_tokens(content[window.start : window.end]) & tokens
            new_tokens = window_tokens - covered
            adjusted = window.score + len(new_tokens) * 1.25 - len(window_tokens & covered) * 0.2
            if adjusted > best_adjusted:
                best = window
                best_adjusted = adjusted
        if best is None:
            break
        selected.append(best)
        covered |= query_tokens(content[best.start : best.end]) & tokens
        candidates = [window for window in candidates if window != best]
    return selected


def question_aware_snippet(content: str, question: str, max_chars: int) -> str:
    if max_chars <= 0 or len(content) <= max_chars:
        return content[:max(0, max_chars)]

    tokens = query_tokens(question)
    if not tokens:
        return content[:max_chars]

    leading_budget = min(260, max_chars // 6)
    window_chars = min(760, max(360, (max_chars - 120) // 3))
    ranges = _window_ranges(len(content), window_chars) + _line_context_ranges(content, window_chars)
    scored = [
        ScoredWindow(
            score=_score_window(content[start:end], tokens, question),
            start=start,
            end=end,
        )
        for start, end in ranges
    ]
    scored.sort(key=lambda item: (-item.score, item.start))
    selected = _select_diverse_windows(scored, content, tokens, limit=5)

    if not selected:
        return content[:max_chars]

    parts: list[str] = []
    selected_by_position = sorted(selected, key=lambda item: item.start)
    leading = content[:leading_budget].strip()
    if leading and selected_by_position[0].start > leading_budget:
        parts.append("[Document start]\n" + leading)

    for index, window in enumerate(selected_by_position, 1):
        text = content[window.start : window.end].strip()
        if not text or text in leading:
            continue
        current = "\n\n".join(parts)
        label = f"[Relevant window {index}]\n"
        remaining = max_chars - len(current) - (2 if current else 0) - len(label)
        if remaining < 160:
            break
        if len(text) > remaining:
            text = text[:remaining].strip()
        if text:
            parts.append(f"[Relevant window {index}]\n{text}")
    return "\n\n".join(parts)[:max_chars]
