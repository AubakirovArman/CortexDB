#!/usr/bin/env python3
"""Confluence collection selector for EnterpriseRAG-Bench.

This deterministic postprocess targets completeness questions that ask for a
collection of Confluence documents, such as case studies or incident
postmortems. It uses only question text, source type metadata, baseline
document ids, candidate document ids, and document paths. It does not call an
LLM/API and does not use gold labels to select documents.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        values[qid] = row
    return values


def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") != "completeness":
        return None
    source_types = {str(item) for item in question.get("source_types", [])}
    if "confluence" not in source_types:
        return None

    text = str(question.get("question", "")).lower()
    if (
        "case studies" in text
        or "customer stories" in text
        or "one-page success stories" in text
    ):
        return "case_studies"
    if (
        "postmortem" in text
        or "postmortems" in text
        or "incident writeups" in text
        or "internal incident writeups" in text
        or "production incidents" in text
    ):
        return "postmortems"
    return None


def qualifies(mode: str, path: str) -> bool:
    if not path.startswith("confluence/"):
        return False
    if mode == "case_studies":
        return "/case-studies/" in path
    if mode == "postmortems":
        return "/postmortems/" in path or "/incident-review/" in path
    return False


def select_docs(
    *,
    mode: str,
    baseline_ids: list[str],
    candidate_ids: list[str],
    uuid_index: dict[str, str],
    limit: int,
    protect_baseline_prefix: int,
    candidate_rank_limit: int,
) -> tuple[list[str], list[dict[str, Any]]]:
    selected: list[str] = []
    seen: set[str] = set()
    diagnostics: list[dict[str, Any]] = []

    for doc_id in baseline_ids[: min(protect_baseline_prefix, limit)]:
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)

    for rank, doc_id in enumerate(candidate_ids[:candidate_rank_limit], 1):
        if len(selected) >= limit:
            break
        if doc_id in seen:
            continue
        path = str(uuid_index.get(doc_id, ""))
        if not qualifies(mode, path):
            continue
        selected.append(doc_id)
        seen.add(doc_id)
        diagnostics.append({"doc_id": doc_id, "path": path, "rank": rank})

    for doc_id in baseline_ids:
        if len(selected) >= limit:
            break
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)

    return selected[:limit], diagnostics


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    candidates = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidates")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")

    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    changed_rows = 0
    routed_rows = 0
    mode_counts: dict[str, int] = {}
    diagnostics: dict[str, Any] = {}

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        output = dict(base_row)
        mode = selector_mode(question)
        if mode:
            routed_rows += 1
            mode_counts[mode] = mode_counts.get(mode, 0) + 1
            selected, selected_diagnostics = select_docs(
                mode=mode,
                baseline_ids=baseline_ids,
                candidate_ids=doc_ids(candidates.get(qid)),
                uuid_index=uuid_index,
                limit=args.limit,
                protect_baseline_prefix=args.protect_baseline_prefix,
                candidate_rank_limit=args.candidate_rank_limit,
            )
            if selected != baseline_ids:
                changed_rows += 1
            output["document_ids"] = selected
            output["route"] = {
                "candidate_rank_limit": args.candidate_rank_limit,
                "enabled": True,
                "mode": mode,
                "policy": args.policy_name,
                "protect_baseline_prefix": args.protect_baseline_prefix,
                "source": "confluence_collection_selector",
            }
            if args.diagnostics:
                diagnostics[qid] = {
                    "baseline": baseline_ids,
                    "mode": mode,
                    "selected": selected,
                    "selected_candidates": selected_diagnostics,
                }
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {
                "enabled": False,
                "policy": args.policy_name,
                "source": "confluence_collection_selector",
            }
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_rank_limit": args.candidate_rank_limit,
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "changed_rows": changed_rows,
        "diagnostics": diagnostics,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "mode_counts": mode_counts,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware selection.",
        "output": str(args.output),
        "policy_name": args.policy_name,
        "protect_baseline_prefix": args.protect_baseline_prefix,
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "routed_rows": routed_rows,
        "schema_version": "cortexdb.enterprise_rag_bench.confluence_collection_selector.v1",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="confluence_collection_selector_v1")
    parser.add_argument("--candidate-rank-limit", type=int, default=400)
    parser.add_argument("--protect-baseline-prefix", type=int, default=2)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--diagnostics", action="store_true")
    args = parser.parse_args()
    if args.candidate_rank_limit <= 0:
        parser.error("--candidate-rank-limit must be positive")
    if args.protect_baseline_prefix < 0:
        parser.error("--protect-baseline-prefix must be non-negative")
    if args.limit <= 0:
        parser.error("--limit must be positive")
    if args.protect_baseline_prefix > args.limit:
        parser.error("--protect-baseline-prefix cannot exceed --limit")
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
