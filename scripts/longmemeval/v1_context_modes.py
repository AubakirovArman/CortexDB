#!/usr/bin/env python3
"""Context rendering helpers for LongMemEval v1 retrieval logs."""

from __future__ import annotations

import re
from typing import Any


def normalize_ws(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip()


def clip_middle(value: str, limit: int) -> str:
    if limit <= 0 or len(value) <= limit:
        return value
    marker = " ... "
    if limit <= len(marker):
        return value[:limit]
    keep = limit - len(marker)
    head = keep // 2
    tail = keep - head
    return value[:head].rstrip() + marker + value[-tail:].lstrip()


def turn_text(turn: dict[str, Any], mode: str, max_turn_chars: int) -> str:
    role = str(turn.get("role", "unknown"))
    content = normalize_ws(str(turn.get("content", "")))
    if not content:
        return ""
    if mode == "user":
        return content if role == "user" else ""
    if mode == "conversation":
        return f"{role}: {content}"
    if mode == "compact":
        return f"{role}: {clip_middle(content, max_turn_chars)}"
    raise RuntimeError(f"unsupported text mode: {mode}")


def session_text(
    session: list[dict[str, Any]],
    mode: str,
    max_turn_chars: int,
    max_session_chars: int,
) -> str:
    parts = [turn_text(turn, mode, max_turn_chars) for turn in session]
    text = "\n".join(part for part in parts if part)
    if mode == "compact":
        return clip_middle(text, max_session_chars)
    return text
