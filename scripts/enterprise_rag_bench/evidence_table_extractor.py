"""Deterministic evidence-table extraction for EnterpriseRAG-Bench answers."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from question_decomposition import precise_anchors, tokens


SCHEMA_VERSION = "cortexdb.enterprise_rag_bench.evidence_table.v1"

FACT_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "number",
        re.compile(
            r"(?:[$€£]\s*)?\b\d+(?:[.,]\d+)?\s*(?:%|ms|s|sec|seconds?|minutes?|hours?|"
            r"days?|weeks?|months?|mib|gib|gb|mb|kb|rps|qps|requests?|credits?|users?|"
            r"files?|transcripts?|tickets?|usd|eur|kzt)?\b",
            re.IGNORECASE,
        ),
    ),
    (
        "date",
        re.compile(
            r"\b(?:\d{4}-\d{2}-\d{2}|\d{1,2}/\d{1,2}/\d{2,4}|"
            r"(?:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\s+\d{1,2},?\s+\d{4})\b",
            re.IGNORECASE,
        ),
    ),
    (
        "literal",
        re.compile(
            r"`[^`]{2,120}`|\b[A-Z][A-Za-z0-9]*-[0-9]{2,}\b|\bX-[A-Za-z0-9-]{2,}\b|"
            r"\b[A-Za-z0-9_./:-]+\.(?:json|ya?ml|toml|md|txt|zip|go|rs|py|ts|tsx|js|sql)\b|"
            r"/[A-Za-z0-9_./:-]{4,}",
        ),
    ),
)

MARKER_TYPES = {
    "approver": "person_or_role",
    "approval": "person_or_role",
    "assignee": "owner",
    "blocker": "status",
    "blocked": "status",
    "cause": "cause",
    "default": "default",
    "delay": "risk",
    "delayed": "risk",
    "deadline": "time",
    "dependency": "risk",
    "dri": "owner",
    "fix": "fix",
    "fixed": "fix",
    "follow-up": "fix",
    "guardrail": "fix",
    "incident": "status",
    "lead": "owner",
    "limit": "threshold",
    "mitigation": "fix",
    "next action": "fix",
    "next step": "fix",
    "owner": "owner",
    "p95": "metric",
    "p99": "metric",
    "policy": "policy",
    "required": "requirement",
    "remediation": "fix",
    "responsible": "owner",
    "risk": "risk",
    "rollback": "status",
    "root cause": "cause",
    "slo": "metric",
    "status": "status",
    "threshold": "threshold",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def extract_document_content(doc: dict[str, Any]) -> tuple[str, str]:
    title_field = doc.get("title_field_name")
    content_fields = doc.get("content_field_names")
    if not isinstance(title_field, str) or title_field not in doc:
        return ("", json.dumps(doc, ensure_ascii=False))
    title = str(doc.get(title_field, ""))
    if not isinstance(content_fields, list) or not content_fields:
        return (title, json.dumps(doc, ensure_ascii=False))
    parts: list[str] = []
    for field in content_fields:
        if not isinstance(field, str) or field not in doc:
            continue
        value = doc[field]
        if isinstance(value, list):
            value = "\n".join(str(item) for item in value)
        elif isinstance(value, dict):
            value = json.dumps(value, ensure_ascii=False)
        parts.append(f"{field}:\n{value}" if len(content_fields) > 1 else str(value))
    return (title, "\n\n".join(parts))


def _clean(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def _split_table_row(line: str) -> list[str]:
    if "|" not in line:
        return []
    stripped = line.strip()
    if stripped.startswith("|"):
        stripped = stripped[1:]
    if stripped.endswith("|"):
        stripped = stripped[:-1]
    cells = [_clean(cell) for cell in stripped.split("|")]
    cells = [cell for cell in cells if cell]
    return cells if len(cells) >= 2 else []


def _is_table_separator(cells: list[str]) -> bool:
    if not cells:
        return False
    return all(re.fullmatch(r":?-{2,}:?", cell.replace(" ", "")) for cell in cells)


def _structured_table_rows(content: str) -> list[tuple[int, str, list[dict[str, str]]]]:
    lines = content.replace("\\n", "\n").splitlines()
    rows: list[tuple[int, str, list[dict[str, str]]]] = []
    index = 0
    while index + 1 < len(lines):
        header = _split_table_row(lines[index])
        separator = _split_table_row(lines[index + 1])
        if not header or not _is_table_separator(separator):
            index += 1
            continue
        row_index = index + 2
        while row_index < len(lines):
            values = _split_table_row(lines[row_index])
            if not values or _is_table_separator(values):
                break
            cells = []
            for header_index, header_name in enumerate(header):
                value = values[header_index] if header_index < len(values) else ""
                if value:
                    cells.append({"header": header_name, "value": value})
            if cells:
                text = " | ".join(f"{cell['header']}: {cell['value']}" for cell in cells)
                rows.append((row_index + 1, text, cells))
            row_index += 1
        index = max(row_index, index + 1)
    return rows


def _line_facts(line: str) -> list[str]:
    facts: list[str] = []
    for fact_type, pattern in FACT_PATTERNS:
        if pattern.search(line):
            facts.append(fact_type)
    lowered = line.lower()
    for marker, fact_type in MARKER_TYPES.items():
        if marker in lowered:
            facts.append(fact_type)
    if "|" in line and len(line.split("|")) >= 3:
        facts.append("table_row")
    return sorted(set(facts))


def _segments(content: str) -> list[tuple[int, str]]:
    segments: list[tuple[int, str]] = []
    for line_number, raw_line in enumerate(content.replace("\\n", "\n").splitlines(), 1):
        line = _clean(raw_line)
        if len(line) <= 520:
            segments.append((line_number, line))
            continue
        parts = re.split(r"(?<=[.!?])\s+(?=[A-Z0-9`/])", line)
        for part in parts:
            cleaned = _clean(part)
            if cleaned:
                segments.append((line_number, cleaned))
    return segments


def _score_line(line: str, question_tokens: set[str], anchors: list[str], fact_types: list[str]) -> float:
    lowered = line.lower()
    line_tokens = set(tokens(line))
    score = float(len(question_tokens & line_tokens) * 2)
    for anchor in anchors:
        if anchor and anchor.lower() in lowered:
            score += 5.0
    score += len(fact_types) * 2.0
    if any(fact_type in {"number", "date", "literal", "table_row"} for fact_type in fact_types):
        score += 2.0
    if len(line) <= 240:
        score += 0.5
    return score


def extract_evidence_table(
    *,
    doc_id: str,
    title: str,
    content: str,
    question: str,
    max_facts: int = 8,
) -> list[dict[str, Any]]:
    question_tokens = set(tokens(question))
    anchors = precise_anchors(question)[:12]
    rows: list[dict[str, Any]] = []
    seen: set[str] = set()
    for line_number, row_text, cells in _structured_table_rows(content):
        fact_types = sorted(set(_line_facts(row_text) + ["table_row", "structured_table_row"]))
        score = _score_line(row_text, question_tokens, anchors, fact_types) + 4.0
        if score <= 2.0:
            continue
        text = row_text[:420].rstrip()
        key = f"structured:{text.lower()}"
        if key in seen:
            continue
        seen.add(key)
        rows.append(
            {
                "doc_id": doc_id,
                "title": title,
                "line": line_number,
                "fact_types": fact_types,
                "score": round(score, 2),
                "text": text,
                "table_cells": cells,
            }
        )
    for line_number, line in _segments(content):
        if len(line) < 8:
            continue
        fact_types = _line_facts(line)
        if not fact_types:
            continue
        score = _score_line(line, question_tokens, anchors, fact_types)
        if score <= 2.0:
            continue
        text = line[:420].rstrip()
        key = text.lower()
        if key in seen:
            continue
        seen.add(key)
        rows.append(
            {
                "doc_id": doc_id,
                "title": title,
                "line": line_number,
                "fact_types": fact_types,
                "score": round(score, 2),
                "text": text,
            }
        )
    rows.sort(key=lambda item: (-float(item["score"]), int(item["line"])))
    return sorted(rows[:max_facts], key=lambda item: (str(item["doc_id"]), int(item["line"])))


def format_evidence_table_for_prompt(table: dict[str, Any] | None, max_rows: int = 40) -> str:
    if not table:
        return ""
    facts = [fact for fact in table.get("facts", []) if isinstance(fact, dict)]
    if not facts:
        return ""
    lines = [
        "Evidence table: exact candidate facts extracted before answer generation.",
        "For project, rollout, incident, migration, owner, status, blocker, risk, cause, or fix questions,",
        "use these rows as a navigation aid. Trust a row only when its source and text match the question anchors,",
        "then verify the fact against the retrieved document windows before answering.",
    ]
    for fact in facts[:max_rows]:
        fact_types = ",".join(str(item) for item in fact.get("fact_types", []))
        title = str(fact.get("title") or "").strip()
        source = f"{fact.get('doc_id')}"
        if title:
            source = f"{source} ({title[:120]})"
        table_cells = fact.get("table_cells")
        if isinstance(table_cells, list) and table_cells:
            cells = "; ".join(
                f"{cell.get('header')}={cell.get('value')}"
                for cell in table_cells[:8]
                if isinstance(cell, dict)
            )
            text = f"{cells} | raw: {fact.get('text')}"
        else:
            text = fact.get("text")
        lines.append(
            "- source={source} line={line} slot={fact_types}: {text}".format(
                source=source,
                line=fact.get("line"),
                fact_types=fact_types,
                text=text,
            )
        )
    lines.append("Use this table as high-priority evidence, then verify against retrieved documents.")
    return "\n".join(lines)
