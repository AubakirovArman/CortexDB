"""Question-anchored evidence digest for EnterpriseRAG-Bench QA prompts."""

from __future__ import annotations

import re
from dataclasses import dataclass

from context_windows import query_tokens


@dataclass(frozen=True)
class DigestBlock:
    score: float
    start_line: int
    text: str


def _clean(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def _normalize_lines(content: str) -> list[str]:
    return content.replace("\\n", "\n").splitlines()


def _question_phrases(question: str) -> list[str]:
    phrases: list[str] = []
    for phrase in re.findall(r"[A-Za-z0-9_./:-]+(?:\s+[A-Za-z0-9_./:-]+){1,5}", question):
        phrase = phrase.lower().strip()
        if len(phrase) >= 10:
            phrases.append(phrase)
    return phrases


def _line_ranges(lines: list[str]) -> list[tuple[int, int]]:
    ranges: list[tuple[int, int]] = []
    for index, line in enumerate(lines):
        if not line.strip():
            continue
        start = max(0, index - 1)
        end = min(len(lines), index + 4)
        ranges.append((start, end))
    return ranges


def _score_block(text: str, tokens: set[str], phrases: list[str]) -> float:
    lowered = text.lower()
    block_tokens = query_tokens(text)
    overlap = tokens & block_tokens
    score = float(len(overlap) * 2)
    for token in overlap:
        if any(char.isdigit() for char in token):
            score += 2.5
        if any(char in token for char in "/._:-"):
            score += 1.5
        if len(token) >= 8:
            score += 0.5
    for phrase in phrases:
        if phrase in lowered:
            score += 4.0
    for marker, bonus in {
        "|": 2.0,
        "$": 3.0,
        "%": 2.0,
        "miB": 2.0,
        "ms": 1.5,
        "p95": 2.0,
        "p99": 2.0,
        "root cause": 3.0,
        "mitigation": 3.0,
        "allowed headers": 3.0,
        "default": 1.5,
        "updated": 1.5,
        "faq": 1.5,
        "policy": 1.0,
        "canary": 1.5,
    }.items():
        if marker.lower() in lowered:
            score += bonus
    return score


def evidence_digest(content: str, title: str, question: str, max_chars: int = 1400) -> str:
    """Return compact evidence blocks likely to contain exact answer facts."""
    tokens = query_tokens(question)
    if not tokens or max_chars <= 0:
        return ""
    phrases = _question_phrases(question)
    lines = _normalize_lines(content)
    blocks: list[DigestBlock] = []
    for start, end in _line_ranges(lines):
        text = _clean("\n".join(lines[start:end]))
        if not text:
            continue
        score = _score_block(text, tokens, phrases)
        if score > 0:
            blocks.append(DigestBlock(score=score, start_line=start, text=text))

    blocks.sort(key=lambda item: (-item.score, item.start_line))
    selected: list[DigestBlock] = []
    seen: set[str] = set()
    for block in blocks:
        key = block.text[:180].lower()
        if key in seen:
            continue
        if any(block.start_line <= kept.start_line + 1 for kept in selected):
            continue
        selected.append(block)
        seen.add(key)
        if len(selected) >= 5:
            break

    if not selected:
        return ""
    parts = [f"Evidence digest for title: {title}"]
    for block in sorted(selected, key=lambda item: item.start_line):
        snippet = block.text[:360].rstrip()
        parts.append(f"- {snippet}")
    return "\n".join(parts)[:max_chars].strip()
