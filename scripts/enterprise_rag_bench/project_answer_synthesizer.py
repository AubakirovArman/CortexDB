#!/usr/bin/env python3
"""Build oracle-free project answer cards from retrieved documents."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

from evidence_slot_planner import build_evidence_plan
from question_decomposition import precise_anchors, tokens


SCHEMA_VERSION = "cortexdb.enterprise_rag_bench.project_answer_card.v1"

CATEGORY_MARKERS: dict[str, tuple[str, ...]] = {
    "identity": (
        "project",
        "incident",
        "ticket",
        "account",
        "tenant",
        "customer",
        "workspace",
        "product",
    ),
    "status": (
        "status",
        "state",
        "blocked",
        "blocker",
        "resolved",
        "shipped",
        "rolled back",
        "deployed",
        "pending",
    ),
    "owner": (
        "owner",
        "dri",
        "assignee",
        "lead",
        "approver",
        "reviewer",
        "responsible",
        "team",
    ),
    "timeline": (
        "deadline",
        "eta",
        "scheduled",
        "window",
        "date",
        "by ",
        "before",
        "after",
        "week",
        "month",
    ),
    "risk": (
        "risk",
        "dependency",
        "delay",
        "delayed",
        "failure",
        "outage",
        "root cause",
        "cause",
        "regression",
    ),
    "action": (
        "mitigation",
        "remediation",
        "fix",
        "next action",
        "next step",
        "rollback",
        "guardrail",
        "runbook",
        "verify",
    ),
    "metric": (
        "threshold",
        "limit",
        "p95",
        "p99",
        "sla",
        "slo",
        "ms",
        "%",
        "rate",
    ),
}

ARTIFACT_RE = re.compile(
    r"`[^`]{2,120}`|\b[A-Z][A-Za-z0-9]*-[0-9]{2,}\b|\b(?:PR|MR|RFC|INC|BUG|TASK)[-_#]?[0-9]{1,6}\b|"
    r"\bgithub\.com/[A-Za-z0-9_.:/-]+\b|\b[A-Za-z0-9_./:-]+\.(?:md|json|ya?ml|toml|rs|py|ts|tsx|go)\b|"
    r"/[A-Za-z0-9_./:-]{4,}",
    re.IGNORECASE,
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


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


def clean_line(text: str, max_chars: int = 360) -> str:
    cleaned = re.sub(r"\s+", " ", text).strip()
    if len(cleaned) <= max_chars:
        return cleaned
    return cleaned[: max(0, max_chars - 4)].rstrip() + " ..."


def line_segments(content: str) -> list[tuple[int, str]]:
    segments: list[tuple[int, str]] = []
    for line_number, raw_line in enumerate(content.replace("\\n", "\n").splitlines(), 1):
        line = clean_line(raw_line, 900)
        if not line:
            continue
        if len(line) <= 520:
            segments.append((line_number, line))
            continue
        for part in re.split(r"(?<=[.!?])\s+(?=[A-Z0-9`/])", line):
            cleaned = clean_line(part, 520)
            if cleaned:
                segments.append((line_number, cleaned))
    return segments


def classify_line(line: str) -> list[str]:
    lowered = line.lower()
    categories = [
        category
        for category, markers in CATEGORY_MARKERS.items()
        if any(marker in lowered for marker in markers)
    ]
    if ARTIFACT_RE.search(line):
        categories.append("linked_artifact")
    if re.search(r"\b\d{4}-\d{2}-\d{2}\b|\b\d+(?:\.\d+)?(?:%|ms|s|mib|gib|gb|mb)?\b", lowered):
        categories.append("metric")
    return sorted(set(categories))


def score_line(line: str, question_tokens: set[str], anchors: list[str], categories: list[str]) -> float:
    lowered = line.lower()
    line_tokens = set(tokens(line))
    score = float(len(question_tokens & line_tokens) * 2)
    for anchor in anchors:
        if anchor and anchor.lower() in lowered:
            score += 6.0
    score += len(categories) * 2.5
    if "status" in categories or "action" in categories:
        score += 2.0
    if "linked_artifact" in categories:
        score += 1.5
    return score


def build_project_card(
    row: dict[str, Any],
    uuid_index: dict[str, str],
    sources_dir: Path,
    *,
    top_docs: int,
    max_rows_per_doc: int,
    max_rows_total: int,
) -> dict[str, Any]:
    question = str(row.get("question") or "")
    qid = str(row.get("question_id") or "")
    doc_ids = [str(item) for item in row.get("document_ids", []) or []]
    anchors = precise_anchors(question)[:12]
    question_tokens = set(tokens(question))
    rows: list[dict[str, Any]] = []

    for rank, doc_id in enumerate(doc_ids[:top_docs], 1):
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        doc_rows: list[dict[str, Any]] = []
        for line_number, line in line_segments(content):
            categories = classify_line(line)
            if not categories:
                continue
            score = score_line(line, question_tokens, anchors, categories)
            if score < 4.5:
                continue
            doc_rows.append(
                {
                    "doc_id": doc_id,
                    "doc_rank": rank,
                    "title": title,
                    "line": line_number,
                    "categories": categories,
                    "score": round(score, 2),
                    "text": clean_line(line),
                }
            )
        doc_rows.sort(key=lambda item: (-float(item["score"]), int(item["line"])))
        rows.extend(doc_rows[:max_rows_per_doc])

    rows.sort(key=lambda item: (-float(item["score"]), int(item["doc_rank"]), int(item["line"])))
    selected = rows[:max_rows_total]
    by_category: dict[str, list[dict[str, Any]]] = {}
    for item in selected:
        for category in item["categories"]:
            by_category.setdefault(category, []).append(item)

    return {
        **build_evidence_plan({"question_id": qid, "question": question}),
        "schema_version": SCHEMA_VERSION,
        "planner": "project_answer_synthesizer",
        "project_card": {
            "identity_anchors": anchors,
            "retrieved_doc_count": len(doc_ids),
            "planned_doc_count": min(len(doc_ids), top_docs),
            "rows": selected,
            "by_category": {key: value[:5] for key, value in sorted(by_category.items())},
            "missing_categories": [
                category
                for category in ("identity", "status", "owner", "timeline", "risk", "action")
                if category not in by_category
            ],
            "answer_policy": (
                "Build the final answer from this card first: identity, status, owner, timeline, blockers, risks, actions, and linked artifacts. "
                "Do not add project details that are absent from the card and retrieved documents."
            ),
        },
    }


def build_report(rows: list[dict[str, Any]], output_jsonl: Path, report_path: Path) -> dict[str, Any]:
    category_counts: Counter[str] = Counter()
    row_counts = []
    missing_counts: Counter[str] = Counter()
    for row in rows:
        card = row.get("project_card", {})
        card_rows = card.get("rows", []) if isinstance(card, dict) else []
        row_counts.append(len(card_rows))
        for card_row in card_rows:
            for category in card_row.get("categories", []):
                category_counts[str(category)] += 1
        for category in card.get("missing_categories", []) if isinstance(card, dict) else []:
            missing_counts[str(category)] += 1
    return {
        "schema_version": "cortexdb.enterprise_rag_bench.project_answer_card_report.v1",
        "questions": len(rows),
        "output_jsonl": str(output_jsonl),
        "report": str(report_path),
        "cards_with_rows": sum(1 for count in row_counts if count > 0),
        "average_rows_per_card": round(sum(row_counts) / len(row_counts), 2) if row_counts else 0.0,
        "by_category": dict(sorted(category_counts.items())),
        "missing_categories": dict(sorted(missing_counts.items())),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-docs", type=int, default=10)
    parser.add_argument("--max-rows-per-doc", type=int, default=5)
    parser.add_argument("--max-rows-total", type=int, default=28)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.top_docs <= 0:
        parser.error("--top-docs must be positive")
    if args.max_rows_per_doc <= 0:
        parser.error("--max-rows-per-doc must be positive")
    if args.max_rows_total <= 0:
        parser.error("--max-rows-total must be positive")
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    args = parse_args()
    retrieval_rows = read_jsonl(args.retrieval_file)
    if args.limit is not None:
        retrieval_rows = retrieval_rows[: args.limit]
    uuid_index = read_json(args.uuid_index)
    rows = [
        build_project_card(
            row,
            uuid_index,
            args.sources_dir,
            top_docs=args.top_docs,
            max_rows_per_doc=args.max_rows_per_doc,
            max_rows_total=args.max_rows_total,
        )
        for row in retrieval_rows
    ]
    report = build_report(rows, args.output_jsonl, args.report)
    write_jsonl(args.output_jsonl, rows)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
