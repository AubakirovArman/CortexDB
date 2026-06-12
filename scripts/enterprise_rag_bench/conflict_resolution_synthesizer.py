#!/usr/bin/env python3
"""Build oracle-free conflict-resolution evidence plans from retrieved docs."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

from answer_guard import concrete_markers
from evidence_slot_planner import build_evidence_plan
from question_decomposition import precise_anchors, tokens


SCHEMA_VERSION = "cortexdb.enterprise_rag_bench.conflict_resolution_plan.v1"

CURRENT_MARKERS = (
    "current",
    "currently",
    "latest",
    "updated",
    "now",
    "as of",
    "default",
    "standard",
    "required",
    "approved",
)
PREVIOUS_MARKERS = (
    "previous",
    "previously",
    "old",
    "older",
    "legacy",
    "prior",
    "before",
    "was",
)
CONFLICT_MARKERS = (
    "conflict",
    "conflicting",
    "contradict",
    "discrepancy",
    "changed",
    "instead",
    "supersedes",
    "superseded",
    "replaced",
    "overrides",
)

DATE_RE = re.compile(
    r"\b(?:20\d{2}-\d{2}-\d{2}|20\d{2}/\d{1,2}/\d{1,2}|"
    r"(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\s+\d{1,2},?\s+20\d{2}|20\d{2})\b",
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


def clean_line(text: str, max_chars: int = 420) -> str:
    cleaned = re.sub(r"\s+", " ", text).strip()
    if len(cleaned) <= max_chars:
        return cleaned
    return cleaned[: max(0, max_chars - 4)].rstrip() + " ..."


def line_segments(content: str) -> list[tuple[int, str]]:
    rows: list[tuple[int, str]] = []
    for line_number, raw_line in enumerate(content.replace("\\n", "\n").splitlines(), 1):
        line = clean_line(raw_line, 900)
        if not line:
            continue
        if len(line) <= 520:
            rows.append((line_number, line))
            continue
        for part in re.split(r"(?<=[.!?])\s+(?=[A-Z0-9`/])", line):
            cleaned = clean_line(part, 520)
            if cleaned:
                rows.append((line_number, cleaned))
    return rows


def claim_kind(line: str) -> str | None:
    lowered = line.lower()
    if any(marker in lowered for marker in CONFLICT_MARKERS):
        return "conflict"
    if any(marker in lowered for marker in CURRENT_MARKERS):
        return "current"
    if any(marker in lowered for marker in PREVIOUS_MARKERS):
        return "previous"
    return None


def date_key(line: str) -> str | None:
    matches = DATE_RE.findall(line)
    if not matches:
        return None
    return str(matches[-1])


def score_line(line: str, question_tokens: set[str], anchors: list[str], kind: str | None) -> float:
    lowered = line.lower()
    line_tokens = set(tokens(line))
    score = float(len(question_tokens & line_tokens) * 2)
    for anchor in anchors:
        if anchor and anchor.lower() in lowered:
            score += 5.0
    if kind:
        score += 4.0
    if concrete_markers(line):
        score += 3.0
    if date_key(line):
        score += 2.0
    return score


def build_conflict_plan(
    row: dict[str, Any],
    uuid_index: dict[str, str],
    sources_dir: Path,
    *,
    top_docs: int,
    max_rows_total: int,
) -> dict[str, Any]:
    question = str(row.get("question") or "")
    qid = str(row.get("question_id") or "")
    doc_ids = [str(item) for item in row.get("document_ids", []) or []]
    anchors = precise_anchors(question)[:12]
    question_tokens = set(tokens(question))
    claims: list[dict[str, Any]] = []

    for rank, doc_id in enumerate(doc_ids[:top_docs], 1):
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        for line_number, line in line_segments(content):
            kind = claim_kind(line)
            markers = concrete_markers(line)
            if not kind and not markers:
                continue
            score = score_line(line, question_tokens, anchors, kind)
            if score < 5.0:
                continue
            claims.append(
                {
                    "doc_id": doc_id,
                    "doc_rank": rank,
                    "title": title,
                    "line": line_number,
                    "kind": kind or "candidate",
                    "date": date_key(line),
                    "markers": markers[:12],
                    "score": round(score, 2),
                    "text": clean_line(line),
                }
            )

    claims.sort(
        key=lambda item: (
            0 if item["kind"] == "current" else 1 if item["kind"] == "conflict" else 2,
            -float(item["score"]),
            int(item["doc_rank"]),
            int(item["line"]),
        )
    )
    selected = claims[:max_rows_total]
    by_kind: dict[str, list[dict[str, Any]]] = {}
    for claim in selected:
        by_kind.setdefault(str(claim["kind"]), []).append(claim)
    return {
        **build_evidence_plan({"question_id": qid, "question": question}),
        "schema_version": SCHEMA_VERSION,
        "planner": "conflict_resolution_synthesizer",
        "conflict_resolution": {
            "anchors": anchors,
            "claims": selected,
            "by_kind": {key: value[:8] for key, value in sorted(by_kind.items())},
            "answer_policy": (
                "If claims conflict, answer with the current/latest/updated value first, then mention previous or conflicting values only when evidence shows them. "
                "Do not merge values from different scenarios."
            ),
        },
    }


def build_report(rows: list[dict[str, Any]], output_jsonl: Path, report_path: Path) -> dict[str, Any]:
    by_kind: Counter[str] = Counter()
    claim_counts = []
    for row in rows:
        conflict = row.get("conflict_resolution", {})
        claims = conflict.get("claims", []) if isinstance(conflict, dict) else []
        claim_counts.append(len(claims))
        for claim in claims:
            by_kind[str(claim.get("kind") or "unknown")] += 1
    return {
        "schema_version": "cortexdb.enterprise_rag_bench.conflict_resolution_report.v1",
        "questions": len(rows),
        "output_jsonl": str(output_jsonl),
        "report": str(report_path),
        "plans_with_claims": sum(1 for count in claim_counts if count > 0),
        "average_claims_per_plan": round(sum(claim_counts) / len(claim_counts), 2) if claim_counts else 0.0,
        "by_kind": dict(sorted(by_kind.items())),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-docs", type=int, default=10)
    parser.add_argument("--max-rows-total", type=int, default=24)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.top_docs <= 0:
        parser.error("--top-docs must be positive")
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
        build_conflict_plan(
            row,
            uuid_index,
            args.sources_dir,
            top_docs=args.top_docs,
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
