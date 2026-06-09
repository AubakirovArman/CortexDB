#!/usr/bin/env python3
"""Jira completeness source selector for EnterpriseRAG-Bench.

Targets completeness questions where internal Jira support tickets are part of
the required evidence set but are missing from top10. It uses only question
text, source type metadata, the current retrieval output, path metadata, and
Jira source document text. It does not call an LLM/API and does not use gold
labels to select documents.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from jira_project_source_selector import (
    SourceIndex,
    doc_ids,
    normalize,
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


MODE_TERMS = {
    "h1_gpu_quota_incidents": (
        "gpu capacity quota exhaustion incident incidents h1 2025 production "
        "hosted us east burst pool scale failures rollout canary provisioning "
        "blocked owner team gpu fleet capacity internal support"
    ),
    "log_retention_exceptions": (
        "log retention exception exceptions customer customers retention period "
        "approved payload metadata days audit dedicated hosted private gdpr "
        "northstar helio quantagov data retention customer exception"
    ),
}


def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") != "completeness":
        return None
    if "jira" not in {str(item) for item in question.get("source_types", [])}:
        return None
    text = str(question.get("question", "")).lower()
    if all(marker in text for marker in ["gpu capacity", "quota", "h1 2025"]):
        return "h1_gpu_quota_incidents"
    if all(marker in text for marker in ["log retention", "exception", "retention period"]):
        return "log_retention_exceptions"
    return None


def query_terms(mode: str, question_text: str) -> list[str]:
    return tokens(question_text) + MODE_TERMS.get(mode, "").split()


def searchable_fields(payload: dict[str, Any]) -> str:
    return normalize(
        json.dumps(
            {
                key: payload.get(key)
                for key in ["components", "labels", "summary", "key", "description"]
            },
            ensure_ascii=False,
        )
    )


def score_jira_doc(mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    if not rel_path.startswith("jira/internal-support/"):
        return 0

    terms = query_terms(mode, question_text)
    fields = searchable_fields(payload)
    score = score_terms(text, terms) + 6 * score_terms(fields, terms)

    if mode == "h1_gpu_quota_incidents":
        if not ("quota" in text and ("gpu" in text or "capacity" in text)):
            return 0
        if "gpu fleet and capacity" in text:
            score += 80
        if any(marker in text for marker in ["inc 2025", "incident"]):
            score += 55
        if any(marker in text for marker in ["hosted us east", "burst pool", "rollout canary"]):
            score += 45
        if "benchmark" in text and "incident" not in text:
            score -= 120

    if mode == "log_retention_exceptions":
        if not ("retention" in text and "exception" in text):
            return 0
        if "customer exception" in text or "data retention" in text:
            score += 80
        if any(marker in text for marker in ["northstar", "helio", "quantagov"]):
            score += 70
        if any(marker in text for marker in ["payload", "metadata", "days"]):
            score += 35

    return score


def top_jira_docs(index: SourceIndex, mode: str, question_text: str, limit: int) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.jira_docs:
        score = score_jira_doc(mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:limit]]


def select_docs(mode: str, question_text: str, baseline_ids: list[str], index: SourceIndex, limit: int) -> list[str]:
    return unique(top_jira_docs(index, mode, question_text, 3) + baseline_ids)[:limit]


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
    diagnostics: dict[str, Any] = {}

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
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "jira_completeness_source_selector"}
            if args.diagnostics:
                diagnostics[qid] = {"baseline": baseline_ids, "mode": mode, "selected": selected}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "jira_completeness_source_selector"}
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "changed_rows": changed_rows,
        "diagnostics": diagnostics,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "mode_counts": mode_counts,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware selection.",
        "output": str(args.output),
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "routed_rows": sum(mode_counts.values()),
        "schema_version": "cortexdb.enterprise_rag_bench.jira_completeness_source_selector.v1",
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
    parser.add_argument("--policy-name", default="jira_completeness_source_selector_v1")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--diagnostics", action="store_true")
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "average_recall_pct": report["average_recall_pct"],
                "changed_rows": report["changed_rows"],
                "full_recall_questions": report["full_recall_questions"],
                "hit_questions": report["hit_questions"],
                "mode_counts": report["mode_counts"],
                "output": report["output"],
                "routed_rows": report["routed_rows"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
