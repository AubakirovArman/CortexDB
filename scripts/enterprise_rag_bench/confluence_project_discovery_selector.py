#!/usr/bin/env python3
"""Confluence project discovery selector for EnterpriseRAG-Bench.

Targets project-related questions where required Confluence source pages are
absent from the broad top1000 candidate pool. It scans local Confluence source
fields deterministically: no LLM/API calls and no gold-aware doc IDs.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from confluence_project_discovery_modes import MODE_CONFIG
from jira_project_source_selector import (
    doc_ids,
    read_json,
    read_jsonl,
    recall_pct,
    rows_by_id,
    score_terms,
    tokens,
    unique,
    write_json,
    write_jsonl,
)


def stringify(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return " ".join(stringify(item) for item in value)
    if isinstance(value, dict):
        return " ".join(f"{key} {stringify(val)}" for key, val in sorted(value.items()))
    return "" if value is None else str(value)


def selector_mode(question: dict[str, Any]) -> str | None:
    question_type = str(question.get("question_type", ""))
    source_types = {str(item) for item in question.get("source_types", [])}
    text = str(question.get("question", "")).lower()
    if "confluence" not in source_types:
        return None
    for mode, config in MODE_CONFIG.items():
        if question_type not in config["type"]:
            continue
        if all(needle in text for needle in config["contains"]):
            return mode
    return None


def confluence_text(rel_path: str, payload: dict[str, Any]) -> str:
    values = [
        rel_path,
        payload.get("title"),
        payload.get("space"),
        payload.get("owner_team"),
        payload.get("status"),
        payload.get("labels"),
        payload.get("related_pages"),
        payload.get("original_location"),
        payload.get("content"),
        payload.get("body"),
    ]
    return " ".join(stringify(value) for value in values if value)


class SourceIndex:
    def __init__(self, *, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.sources_dir = sources_dir
        self.reverse_index = {rel_path: doc_id for doc_id, rel_path in uuid_index.items()}
        self.confluence_paths = [
            rel_path
            for rel_path in sorted(uuid_index.values())
            if rel_path.startswith("confluence/")
        ]
        self._docs: list[tuple[str, str, dict[str, Any], str]] | None = None

    def docs(self) -> list[tuple[str, str, dict[str, Any], str]]:
        if self._docs is not None:
            return self._docs
        docs: list[tuple[str, str, dict[str, Any], str]] = []
        for rel_path in self.confluence_paths:
            path = self.sources_dir / rel_path
            try:
                payload = read_json(path)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                payload = {}
            if not isinstance(payload, dict):
                payload = {}
            docs.append((self.reverse_index.get(rel_path, ""), rel_path, payload, confluence_text(rel_path, payload)))
        self._docs = docs
        return docs


def allowed_for_mode(mode: str, rel_path: str) -> bool:
    return any(rel_path.startswith(prefix) for prefix in MODE_CONFIG[mode]["allowed_paths"])


def quality_gate(mode: str, text: str, rel_path: str) -> bool:
    lower = text.lower()
    path = rel_path.lower()
    if any(marker in path for marker in MODE_CONFIG[mode]["path_bonus"]):
        return True
    if mode == "fast_tier_canary_slo_dashboard":
        return (
            "fast" in lower
            and "slo" in lower
            and "batching" in lower
            and ("dashboard" in lower or "latency targets" in lower)
        )
    if mode == "rollout_split_orchestrator":
        return (
            "rollout" in lower
            and "orchestrator" in lower
            and "traffic" in lower
            and "canary" in lower
        )
    return False


def score_doc(mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    if not allowed_for_mode(mode, rel_path) or not quality_gate(mode, text, rel_path):
        return 0
    config = MODE_CONFIG[mode]
    haystack = text.lower()
    title_path = f"{rel_path} {stringify(payload.get('title'))}".lower()
    terms = tokens(question_text) + str(config["terms"]).split()
    score = score_terms(haystack, terms) + 12 * score_terms(title_path, terms)
    score += 220 * sum(1 for marker in config["path_bonus"] if marker in rel_path.lower())
    return score


def top_confluence_docs(index: SourceIndex, mode: str, question_text: str) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.docs():
        score = score_doc(mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    max_docs = int(MODE_CONFIG[mode]["max_docs"])
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:max_docs]]


def select_docs(mode: str, question_text: str, baseline_ids: list[str], index: SourceIndex, limit: int) -> list[str]:
    return unique(top_confluence_docs(index, mode, question_text) + baseline_ids)[:limit]


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    index = SourceIndex(uuid_index=uuid_index, sources_dir=args.sources_dir)

    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    changed_rows = 0
    mode_counts: dict[str, int] = {}

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        output = dict(base_row)
        mode = selector_mode(question)
        if mode:
            selected = select_docs(mode, str(question.get("question", "")), baseline_ids, index, args.limit)
            changed_rows += int(selected != baseline_ids)
            mode_counts[mode] = mode_counts.get(mode, 0) + 1
            output["document_ids"] = selected
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "confluence_project_discovery_selector"}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "confluence_project_discovery_selector"}
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "changed_rows": changed_rows,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "mode_counts": mode_counts,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware doc IDs.",
        "output": str(args.output),
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "routed_rows": sum(mode_counts.values()),
        "schema_version": "cortexdb.enterprise_rag_bench.confluence_project_discovery_selector.v1",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="confluence_project_discovery_selector_v1")
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    report = run(parse_args())
    keys = ("average_recall_pct", "changed_rows", "full_recall_questions", "hit_questions", "mode_counts", "output", "routed_rows")
    print(json.dumps({key: report[key] for key in keys}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
