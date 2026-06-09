"""Optional evidence artifacts for EnterpriseRAG answer prompts."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from evidence_slot_planner import build_evidence_plan, format_evidence_plan_for_prompt, read_jsonl as read_plan_jsonl
from evidence_table_extractor import (
    extract_document_content,
    extract_evidence_table,
    format_evidence_table_for_prompt,
    read_json,
    read_jsonl as read_table_jsonl,
)


def maps_by_id(path: Path | None, *, kind: str) -> dict[str, dict[str, Any]]:
    if path is None:
        return {}
    reader = read_plan_jsonl if kind == "plan" else read_table_jsonl
    return {
        str(row.get("question_id")): row
        for row in reader(path)
        if row.get("question_id") is not None
    }


def evidence_plan_for_row(
    row: dict[str, Any],
    plans: dict[str, dict[str, Any]],
    include: bool,
) -> dict[str, Any] | None:
    if not include:
        return None
    qid = str(row.get("question_id"))
    return plans.get(qid) or build_evidence_plan(row)


def evidence_table_for_row(
    *,
    row: dict[str, Any],
    tables: dict[str, dict[str, Any]],
    include: bool,
    doc_ids: list[str],
    uuid_index: dict[str, str],
    sources_dir: Path,
    max_facts_per_doc: int,
) -> dict[str, Any] | None:
    if not include:
        return None
    qid = str(row.get("question_id"))
    if qid in tables:
        return tables[qid]
    facts: list[dict[str, Any]] = []
    question = str(row.get("question") or "")
    for doc_id in doc_ids:
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        facts.extend(
            extract_evidence_table(
                doc_id=doc_id,
                title=title,
                content=content,
                question=question,
                max_facts=max_facts_per_doc,
            )
        )
    facts.sort(key=lambda item: (-float(item["score"]), str(item["doc_id"]), int(item["line"])))
    return {"question_id": row.get("question_id"), "facts": facts}


def with_evidence_artifacts(
    prompt: str,
    evidence_plan: dict[str, Any] | None,
    evidence_table: dict[str, Any] | None,
) -> str:
    blocks = [
        block
        for block in (
            format_evidence_plan_for_prompt(evidence_plan) if evidence_plan else "",
            format_evidence_table_for_prompt(evidence_table) if evidence_table else "",
        )
        if block
    ]
    if not blocks:
        return prompt
    artifact_text = "\n\n".join(blocks)
    marker = "\nRetrieved documents:"
    if marker not in prompt:
        return f"{artifact_text}\n\n{prompt}"
    return prompt.replace(marker, f"\n{artifact_text}\n{marker}", 1)
