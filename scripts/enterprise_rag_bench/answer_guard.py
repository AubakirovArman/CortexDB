"""Oracle-free answer guards for EnterpriseRAG-Bench generation."""

from __future__ import annotations

import re
from typing import Any


INSUFFICIENT = "Insufficient information."

_DATE_RE = re.compile(
    r"\b(?:Jan|January|Feb|February|Mar|March|Apr|April|May|Jun|June|Jul|July|"
    r"Aug|August|Sep|Sept|September|Oct|October|Nov|November|Dec|December)"
    r"\s+\d{1,2}(?:,\s*\d{4})?\b",
    re.IGNORECASE,
)
_ISO_DATE_RE = re.compile(r"\b20\d{2}[-/]\d{1,2}[-/]\d{1,2}\b")
_YEAR_RE = re.compile(r"\b20\d{2}\b")
_DECIMAL_RE = re.compile(r"(?<![\w.])\d+\.\d+(?![\w.])")
_NUMBER_WITH_UNIT_RE = re.compile(
    r"(?<![\w.])\$?\d+(?:,\d{3})*(?:\.\d+)?\s*"
    r"(?:%|percent|ms|msec|seconds?|secs?|minutes?|mins?|hours?|hrs?|days?|"
    r"weeks?|months?|years?|kb|mb|gb|tb|kib|mib|gib|tib|k|usd|kzt|eur|gbp|"
    r"qps|rps|req/s|requests?|tokens?|users?|seats?|regions?|nodes?|"
    r"replicas?|shards?|pods?|workers?)\b",
    re.IGNORECASE,
)
_LARGE_NUMBER_RE = re.compile(r"(?<![\w.])\d{2,}(?:,\d{3})*(?![\w.])")
_TICKET_RE = re.compile(r"\b[A-Z][A-Z0-9]{1,12}-\d+\b")
_PATH_RE = re.compile(r"(?<!\w)(?:/[A-Za-z0-9._~+\-/%]+|[A-Za-z0-9_.-]+/[A-Za-z0-9._~+\-/%]+)")
_VERSION_RE = re.compile(r"\bv?\d+\.\d+(?:\.\d+)?(?:-[A-Za-z0-9_.-]+)?\b", re.IGNORECASE)
_CODE_RE = re.compile(r"`([^`]{2,120})`")


def _normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.casefold())


def _expand_k_suffix(match: re.Match) -> str:
    num = match.group(1)
    try:
        value = float(num)
    except ValueError:
        return match.group(0)
    rounded = int(value * 1000)
    return str(rounded)


def _canonicalize(text: str) -> str:
    """Return a normalised form where common numeric/unit variants collide.

    This is intentionally conservative: it only collapses variants that a human
    reader would treat as the same value (MiB/MB, 1.5k/1500, 30 secs/30 sec,
    50 percent/50%). It does not rewrite semantic meaning.
    """

    normalized = _normalize(text)
    # 1.5k / 2k -> 1500 / 2000, but do not touch kb/kib.
    normalized = re.sub(r"(\d+(?:\.\d+)?)\s*k(?!b|ib)", _expand_k_suffix, normalized)
    # Remove thousands separators so 1,500 matches 1500.
    normalized = re.sub(r"(\d),(?=\d{3}\b)", r"\1", normalized)
    # Binary and decimal storage units are treated as equivalent for guarding.
    normalized = re.sub(r"\bmib\b", "mb", normalized)
    normalized = re.sub(r"\bkib\b", "kb", normalized)
    normalized = re.sub(r"\bgib\b", "gb", normalized)
    normalized = re.sub(r"\btib\b", "tb", normalized)
    # Time unit variants -> canonical short form.
    normalized = re.sub(r"\b(seconds?|secs?)\b", "sec", normalized)
    normalized = re.sub(r"\b(minutes?|mins?)\b", "min", normalized)
    normalized = re.sub(r"\b(hours?|hrs?)\b", "hr", normalized)
    normalized = re.sub(r"\b(days?)\b", "day", normalized)
    normalized = re.sub(r"\b(weeks?)\b", "week", normalized)
    normalized = re.sub(r"\b(months?)\b", "month", normalized)
    normalized = re.sub(r"\b(years?)\b", "year", normalized)
    # Percent forms.
    normalized = re.sub(r"\bpercent\b", "%", normalized)
    # Rate/request synonyms.
    normalized = re.sub(r"\brequests?\b", "req", normalized)
    normalized = re.sub(r"\breq/s\b", "rps", normalized)
    normalized = re.sub(r"\bqps\b", "rps", normalized)
    # Collapse spaces between a number and its unit/rate so "10 mb" == "10mb".
    normalized = re.sub(
        r"(\d)\s+(%|ms|msec|sec|min|hr|day|week|month|year|"
        r"kb|mb|gb|tb|rps|req|usd|eur|gbp|kzt|"
        r"tokens?|users?|seats?|regions?|nodes?|replicas?|shards?|pods?|workers?)",
        r"\1\2",
        normalized,
    )
    # Drop trailing .0 so 10.0 matches 10.
    normalized = re.sub(r"(\d)\.0+\b", r"\1", normalized)
    return normalized


def concrete_markers(text: str) -> list[str]:
    """Return exact factual markers that should be visible in evidence."""

    markers: list[str] = []
    for pattern in (
        _CODE_RE,
        _DATE_RE,
        _ISO_DATE_RE,
        _NUMBER_WITH_UNIT_RE,
        _YEAR_RE,
        _DECIMAL_RE,
        _LARGE_NUMBER_RE,
        _TICKET_RE,
        _PATH_RE,
        _VERSION_RE,
    ):
        for match in pattern.finditer(text):
            marker = match.group(1) if pattern is _CODE_RE else match.group(0)
            marker = marker.strip(" ,.;:()[]{}")
            if marker and marker not in markers:
                markers.append(marker)
    return markers


def _unsupported_markers(markers: list[str], evidence_text: str) -> list[str]:
    normalized_haystack = _normalize(evidence_text)
    canonical_haystack = _canonicalize(evidence_text)
    missing: list[str] = []
    for marker in markers:
        normalized = _normalize(marker)
        if normalized and normalized in normalized_haystack:
            continue
        canonical = _canonicalize(marker)
        if canonical and canonical in canonical_haystack:
            continue
        missing.append(marker)
    return missing


def _split_answer(answer: str) -> list[str]:
    stripped = answer.strip()
    if not stripped:
        return []
    lines = [line.strip() for line in stripped.splitlines() if line.strip()]
    if len(lines) > 1:
        return lines
    return [part.strip() for part in re.split(r"(?<=[.!?])\s+", stripped) if part.strip()]


def _repair_statement(statement: str, unsupported: list[str]) -> str:
    """Rewrite one statement without preserving unsupported exact values."""

    if not unsupported:
        return statement
    first_marker = unsupported[0]
    marker_offset = _normalize(statement).find(_normalize(first_marker))
    if marker_offset >= 0:
        prefix = statement[:marker_offset].strip(" ,;:-")
        subject_match = re.search(
            r"(?P<subject>.+?)\s+(?:is|was|are|were|equals|=|:)\s*$",
            prefix,
            re.IGNORECASE,
        )
        if subject_match:
            subject = subject_match.group("subject").strip(" ,;:-")
            if subject:
                return f"{subject} is not stated in the retrieved evidence."

    repaired = statement
    for marker in sorted(unsupported, key=len, reverse=True):
        repaired = re.sub(re.escape(marker), "an unstated value", repaired, flags=re.IGNORECASE)
    repaired = re.sub(r"\s+", " ", repaired).strip()
    repaired = re.sub(r"\ban unstated value\s+(?:days?|seconds?|minutes?|hours?|ms|%)\b", "an unstated value", repaired, flags=re.IGNORECASE)
    repaired = repaired.strip(" ,;:")
    if repaired and repaired != statement:
        if not repaired.endswith((".", "!", "?")):
            repaired += "."
        return repaired
    return "A specific value in this statement is not stated in the retrieved evidence."


def guard_unsupported_claims(
    answer: str,
    evidence_text: str,
    *,
    mode: str = "suppress",
) -> tuple[str, dict[str, Any]]:
    """Report or suppress answer sentences with unsupported concrete markers.

    The guard is deliberately conservative: it only checks exact markers such as
    numbers, dates, IDs, versions, and paths. It does not use benchmark labels or
    gold answers.
    """

    if mode not in {"off", "report", "suppress", "repair"}:
        raise ValueError(f"unsupported guard mode: {mode}")
    if mode == "off" or answer.strip() == INSUFFICIENT:
        return answer, {
            "mode": mode,
            "removed_count": 0,
            "repaired_count": 0,
            "unsupported_markers": [],
        }

    kept: list[str] = []
    removed: list[str] = []
    repaired_statements: list[dict[str, Any]] = []
    unsupported_all: list[str] = []
    flagged_count = 0
    for statement in _split_answer(answer):
        markers = concrete_markers(statement)
        unsupported = _unsupported_markers(markers, evidence_text)
        if unsupported:
            flagged_count += 1
            unsupported_all.extend(unsupported)
            if mode == "report":
                kept.append(statement)
            elif mode == "repair":
                repaired = _repair_statement(statement, unsupported)
                kept.append(repaired)
                repaired_statements.append(
                    {
                        "original": statement,
                        "repaired": repaired,
                        "unsupported_markers": unsupported,
                    }
                )
            else:
                removed.append(statement)
            continue
        kept.append(statement)

    if mode == "suppress" and removed:
        guarded = " ".join(kept).strip()
        if not guarded:
            guarded = INSUFFICIENT
    elif mode == "repair" and repaired_statements:
        guarded = " ".join(kept).strip()
        if not guarded:
            guarded = INSUFFICIENT
    else:
        guarded = answer

    deduped_markers = list(dict.fromkeys(unsupported_all))
    return guarded, {
        "mode": mode,
        "removed_count": len(removed) if mode == "suppress" else 0,
        "repaired_count": len(repaired_statements) if mode == "repair" else 0,
        "flagged_count": flagged_count,
        "unsupported_markers": deduped_markers[:50],
        "repaired_statements": repaired_statements[:25],
    }
