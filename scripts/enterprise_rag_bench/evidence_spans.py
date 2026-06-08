"""Materialized evidence-span extraction for EnterpriseRAG-Bench contexts.

The benchmark uses document IDs, but an agent database should expose smaller
answerable evidence units. This module selects deterministic spans with visible
selection signals so answer prompts can consume evidence, not whole noisy docs.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from context_windows import query_tokens


@dataclass(frozen=True)
class EvidenceSpan:
    score: float
    start: int
    end: int
    text: str
    signals: tuple[str, ...]


def _clean(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def _anchor_terms(question: str) -> set[str]:
    anchors = set(query_tokens(question))
    for pattern in (
        r"`([^`]+)`",
        r"\b[A-Z]{2,}[A-Z0-9_-]*\b",
        r"\b[A-Z][a-z]+(?:[A-Z][a-z0-9]+)+\b",
        r"\b[a-zA-Z0-9_./:-]+\.[a-zA-Z0-9_./:-]+\b",
        r"\b\d+(?:\.\d+)?(?:%|ms|s|mib|gib|gb|mb|hours?|minutes?)?\b",
    ):
        for match in re.findall(pattern, question):
            value = match if isinstance(match, str) else match[0]
            cleaned = value.lower().strip()
            if len(cleaned) > 1:
                anchors.add(cleaned)
    return anchors


def _phrases(question: str) -> list[str]:
    parts = [token for token in query_tokens(question) if len(token) > 2]
    values: set[str] = set()
    for width in (5, 4, 3, 2):
        for index in range(0, max(0, len(parts) - width + 1)):
            phrase = " ".join(parts[index : index + width])
            if len(phrase) >= 12:
                values.add(phrase)
    return sorted(values, key=lambda value: (-len(value), value))


def _line_offsets(content: str) -> list[tuple[int, int, str]]:
    lines: list[tuple[int, int, str]] = []
    offset = 0
    for line in content.replace("\\n", "\n").splitlines(keepends=True):
        lines.append((offset, offset + len(line), line))
        offset += len(line)
    if not lines and content:
        lines.append((0, len(content), content))
    return lines


def _ranges_from_lines(content: str) -> list[tuple[int, int]]:
    lines = _line_offsets(content)
    ranges: list[tuple[int, int]] = []
    for index, (_start, _end, line) in enumerate(lines):
        if not line.strip():
            continue
        start_index = max(0, index - 2)
        end_index = min(len(lines), index + 5)
        ranges.append((lines[start_index][0], lines[end_index - 1][1]))
    return ranges


def _paragraph_ranges(content: str) -> list[tuple[int, int]]:
    ranges: list[tuple[int, int]] = []
    for match in re.finditer(r"(?:^|\n\n)(.*?)(?=\n\n|$)", content, re.DOTALL):
        text = match.group(1)
        if text.strip():
            ranges.append((match.start(1), match.end(1)))
    return ranges


def _sliding_ranges(content: str, width: int = 1200) -> list[tuple[int, int]]:
    if len(content) <= width:
        return [(0, len(content))]
    stride = width // 2
    ranges: list[tuple[int, int]] = []
    start = 0
    while start < len(content):
        end = min(len(content), start + width)
        ranges.append((start, end))
        if end == len(content):
            break
        start += stride
    return ranges


def _signals(text: str, anchors: set[str], phrases: list[str]) -> tuple[float, tuple[str, ...]]:
    lowered = text.lower()
    span_tokens = query_tokens(text)
    overlap = anchors & span_tokens
    signal_names: list[str] = []
    score = float(len(overlap) * 2)

    if overlap:
        signal_names.append("query_overlap")
    if any(any(char.isdigit() for char in token) for token in overlap):
        score += 4.0
        signal_names.append("numeric_anchor")
    if any(any(char in token for char in "/._:-") for token in overlap):
        score += 3.0
        signal_names.append("literal_anchor")
    phrase_hits = sum(1 for phrase in phrases if phrase in lowered)
    if phrase_hits:
        score += phrase_hits * 4.0
        signal_names.append("phrase_match")

    marker_bonuses = {
        "answer": 2.5,
        "answers": 2.5,
        "confirmed": 2.0,
        "required": 2.0,
        "mandatory": 2.0,
        "threshold": 2.0,
        "root cause": 3.5,
        "mitigation": 3.0,
        "decision": 2.0,
        "policy": 1.5,
        "faq": 1.5,
        "updated": 1.5,
        "exception": 1.5,
        "owner": 1.0,
        "approver": 1.0,
    }
    for marker, bonus in marker_bonuses.items():
        if marker in lowered:
            score += bonus
            signal_names.append(marker.replace(" ", "_"))

    if re.search(r"(^|\n)\s*(from|to|sent|subject):", text, re.IGNORECASE):
        score += 2.0
        signal_names.append("email_thread")
    if re.search(r"(^|\n)\s*(q|a|agent|customer|interviewer|speaker)\s*[:\-]", text, re.IGNORECASE):
        score += 2.0
        signal_names.append("dialogue_turn")
    if re.search(r"[$%]|\b(?:p50|p90|p95|p99|ms|mib|gib|gb|mb)\b", lowered):
        score += 2.0
        signal_names.append("metric_value")

    deduped = tuple(dict.fromkeys(signal_names))
    return score, deduped


def _best_anchor_offset(text: str, anchors: set[str], phrases: list[str]) -> int:
    lowered = text.lower()
    candidates: list[tuple[int, int]] = []
    for phrase in phrases:
        offset = lowered.find(phrase)
        if offset >= 0:
            candidates.append((offset, len(phrase) + 20))
    for anchor in anchors:
        offset = lowered.find(anchor.lower())
        if offset >= 0:
            weight = len(anchor) + (20 if any(char.isdigit() for char in anchor) else 0)
            candidates.append((offset, weight))
    for marker in (
        "new metric",
        "metric",
        "answer",
        "confirmed",
        "root cause",
        "mitigation",
        "threshold",
        "required",
    ):
        offset = lowered.find(marker)
        if offset >= 0:
            candidates.append((offset, 10))
    if not candidates:
        return 0
    return max(candidates, key=lambda item: (item[1], -item[0]))[0]


def _trim_around_anchor(
    text: str,
    anchors: set[str],
    phrases: list[str],
    max_chars: int,
) -> str:
    cleaned = _clean(text)
    if len(cleaned) <= max_chars:
        return cleaned
    offset = _best_anchor_offset(cleaned, anchors, phrases)
    start = max(0, offset - max_chars // 3)
    end = min(len(cleaned), start + max_chars)
    start = max(0, end - max_chars)
    if start > 0:
        boundary = cleaned.find(". ", start, min(offset, start + 240))
        if boundary >= 0:
            start = boundary + 2
    if end < len(cleaned):
        boundary = cleaned.rfind(". ", max(start, end - 240), end)
        if boundary > start:
            end = boundary + 1
    prefix = "... " if start > 0 else ""
    suffix = " ..." if end < len(cleaned) else ""
    return f"{prefix}{cleaned[start:end].strip()}{suffix}"


def _candidate_ranges(content: str) -> list[tuple[int, int]]:
    seen: set[tuple[int, int]] = set()
    values: list[tuple[int, int]] = []
    for start, end in (
        _paragraph_ranges(content)
        + _ranges_from_lines(content)
        + _sliding_ranges(content)
    ):
        start = max(0, start)
        end = min(len(content), end)
        if end <= start or (start, end) in seen:
            continue
        seen.add((start, end))
        values.append((start, end))
    return values


def _overlaps(candidate: EvidenceSpan, selected: list[EvidenceSpan]) -> bool:
    for span in selected:
        overlap = max(0, min(candidate.end, span.end) - max(candidate.start, span.start))
        if overlap >= min(candidate.end - candidate.start, span.end - span.start) * 0.30:
            return True
    return False


def select_evidence_spans(
    content: str,
    question: str,
    *,
    max_spans: int = 6,
    max_chars_per_span: int = 1200,
) -> list[EvidenceSpan]:
    anchors = _anchor_terms(question)
    if not anchors:
        return []
    phrases = _phrases(question)
    candidates: list[EvidenceSpan] = []
    for start, end in _candidate_ranges(content):
        raw_text = content[start:end].strip()
        if not raw_text:
            continue
        score, signals = _signals(raw_text, anchors, phrases)
        if score <= 0:
            continue
        text = _trim_around_anchor(raw_text, anchors, phrases, max_chars_per_span)
        candidates.append(EvidenceSpan(score=score, start=start, end=end, text=text, signals=signals))

    candidates.sort(key=lambda item: (-item.score, item.start, item.end))
    selected: list[EvidenceSpan] = []
    seen_text: set[str] = set()
    for candidate in candidates:
        key = candidate.text[:220].lower()
        if key in seen_text or _overlaps(candidate, selected):
            continue
        selected.append(candidate)
        seen_text.add(key)
        if len(selected) >= max_spans:
            break
    return sorted(selected, key=lambda item: item.start)


def evidence_span_context(content: str, title: str, question: str, max_chars: int) -> str:
    spans = select_evidence_spans(content, question)
    if not spans:
        return content[:max_chars]
    parts = [f"Materialized evidence spans for title: {title}"]
    for index, span in enumerate(spans, 1):
        signals = ",".join(span.signals[:6]) or "score"
        block = (
            f"[Evidence span {index} | score={span.score:.2f} | signals={signals}]\n"
            f"{span.text}"
        )
        current = "\n\n".join(parts)
        remaining = max_chars - len(current) - len(block) - 2
        if remaining < 0:
            break
        parts.append(block)
    return "\n\n".join(parts)[:max_chars]
