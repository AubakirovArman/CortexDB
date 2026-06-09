"""Context builders for EnterpriseRAG-Bench answer generation."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from context_windows import question_aware_snippet
from evidence_digest import evidence_digest, evidence_digest_score
from evidence_span_fallback import evidence_span_plus_fallback_context
from evidence_spans import evidence_span_context


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


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


def evidence_first_snippet(content: str, title: str, question: str, max_chars: int) -> str:
    """Prefer compact evidence over broad document windows."""

    if max_chars <= 0:
        return ""
    digest_budget = min(720, max(260, max_chars // 2))
    window_budget = min(520, max(220, max_chars - digest_budget - 80))
    digest = evidence_digest(content, title, question).strip()
    if digest:
        digest = digest[:digest_budget].strip()
    window = question_aware_snippet(content, question, window_budget).strip()
    if digest and window:
        return f"Evidence digest:\n{digest}\n\nSupporting snippet:\n{window}"[:max_chars]
    return (digest or window or content[:max_chars]).strip()[:max_chars]


def document_snippet(content: str, title: str, question: str, max_chars_per_doc: int, context_mode: str) -> str:
    if context_mode == "question-window":
        return question_aware_snippet(content, question, max_chars_per_doc)
    if context_mode == "evidence-spans":
        return evidence_span_context(content, title, question, max_chars_per_doc)
    if context_mode == "span-plus-fallback":
        return evidence_span_plus_fallback_context(content, title, question, max_chars_per_doc)
    if context_mode == "evidence-first":
        return evidence_first_snippet(content, title, question, max_chars_per_doc)
    if context_mode in {"question-window-digest", "question-window-digest-ranked"}:
        digest = evidence_digest(content, title, question)
        snippet_budget = max(1200, max_chars_per_doc - len(digest) - 160)
        snippet = question_aware_snippet(content, question, snippet_budget)
        if digest:
            return f"{digest}\n\nQuestion-aware windows:\n{snippet}"
        return snippet
    return content[:max_chars_per_doc]


def load_context(
    doc_ids: list[str],
    uuid_index: dict[str, str],
    sources_dir: Path,
    max_chars_per_doc: int,
    question: str,
    context_mode: str,
) -> str:
    docs: list[tuple[float, int, str]] = []
    for rank, doc_id in enumerate(doc_ids, 1):
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        snippet = document_snippet(content, title, question, max_chars_per_doc, context_mode)
        score = evidence_digest_score(content, question) if "digest-ranked" in context_mode else 0.0
        docs.append(
            (
                score,
                rank,
                f"--- Document {rank} (ID: {doc_id}) ---\n" f"Title: {title}\n\n{snippet}",
            )
        )
    if "digest-ranked" in context_mode:
        docs.sort(key=lambda item: (-item[0], item[1]))
    return "\n\n".join(text for _, _, text in docs)
