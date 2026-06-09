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
    "blocker": "status",
    "cause": "cause",
    "default": "default",
    "deadline": "time",
    "limit": "threshold",
    "mitigation": "status",
    "owner": "person_or_role",
    "p95": "metric",
    "p99": "metric",
    "policy": "policy",
    "required": "requirement",
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
    for line_number, raw_line in enumerate(content.replace("\\n", "\n").splitlines(), 1):
        line = _clean(raw_line)
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
    lines = ["Evidence table: exact candidate facts extracted before answer generation."]
    for fact in facts[:max_rows]:
        fact_types = ",".join(str(item) for item in fact.get("fact_types", []))
        lines.append(
            "- doc={doc_id} line={line} type={fact_types}: {text}".format(
                doc_id=fact.get("doc_id"),
                line=fact.get("line"),
                fact_types=fact_types,
                text=fact.get("text"),
            )
        )
    lines.append("Use this table as high-priority evidence, then verify against retrieved documents.")
    return "\n".join(lines)
