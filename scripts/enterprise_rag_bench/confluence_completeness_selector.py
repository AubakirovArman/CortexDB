#!/usr/bin/env python3
"""Confluence completeness selector for EnterpriseRAG-Bench.

This deterministic postprocess targets completeness questions whose source set
is exactly Confluence. It uses only question text plus candidate document
path/title fields, with a small synonym map for enterprise wording such as
`communications -> comms`. It does not call an LLM/API and does not use gold
labels to select documents.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

from multi_index_candidate_generation import extract_document_content
from question_decomposition import precise_anchors, tokens


QUERY_EXPANSIONS = {
    "approval": ["approvals", "signoff", "sign-off"],
    "approvals": ["approval", "signoff", "sign-off"],
    "change": ["changes", "change-management"],
    "communication": ["communications", "comms", "status", "status-page"],
    "communications": ["communication", "comms", "status", "status-page"],
    "customer": ["customer-facing"],
    "deployment": ["deploy", "deployments"],
    "gate": ["gating", "go-no-go", "signoff", "sign-off"],
    "gates": ["gating", "go-no-go", "signoff", "sign-off"],
    "hotfix": ["emergency", "serving-runtime"],
    "rollback": ["rollbacks", "rollout", "fallback"],
    "upgrade": ["upgrades", "go-no-go"],
}


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


def normalize(text: str) -> str:
    return re.sub(r"[^a-z0-9_./:%-]+", " ", text.lower()).strip()


def expanded_question_terms(question_text: str) -> set[str]:
    values = tokens(question_text)
    for token in list(values):
        values.extend(QUERY_EXPANSIONS.get(token, []))
    return set(values)


def is_routed(question: dict[str, Any]) -> bool:
    return (
        question.get("question_type") == "completeness"
        and question.get("source_types") == ["confluence"]
    )


class TitleCache:
    def __init__(self, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.values: dict[str, tuple[str, str]] = {}

    def get(self, doc_id: str) -> tuple[str, str]:
        if doc_id in self.values:
            return self.values[doc_id]
        rel_path = str(self.uuid_index.get(doc_id, ""))
        title = ""
        if rel_path:
            try:
                loaded = read_json(self.sources_dir / rel_path)
                if isinstance(loaded, dict):
                    title, _content = extract_document_content(loaded)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                title = ""
        self.values[doc_id] = (rel_path, title)
        return self.values[doc_id]


def score_candidate(
    *,
    question: dict[str, Any],
    doc_id: str,
    rank: int,
    baseline_ids: set[str],
    title_cache: TitleCache,
) -> tuple[float, dict[str, Any]] | None:
    rel_path, title = title_cache.get(doc_id)
    if not rel_path.startswith("confluence/"):
        return None
    question_text = str(question.get("question", ""))
    question_terms = expanded_question_terms(question_text)
    path_text = normalize(rel_path.replace("/", " ").replace("-", " ").replace("_", " "))
    title_text = normalize(title)
    combined_text = f"{path_text} {title_text}"
    path_overlap = len(question_terms & set(tokens(path_text)))
    title_overlap = len(question_terms & set(tokens(title_text)))
    anchor_hits = sum(
        1
        for anchor in precise_anchors(question_text)
        if normalize(anchor) and normalize(anchor) in combined_text
    )
    if path_overlap + title_overlap + anchor_hits == 0:
        return None
    baseline_bonus = 8.0 if doc_id in baseline_ids else 0.0
    score = (
        path_overlap * 12.0
        + title_overlap * 8.0
        + anchor_hits * 20.0
        + 20.0 / math.sqrt(rank)
        + baseline_bonus
    )
    return score, {
        "path": rel_path,
        "rank": rank,
        "path_overlap": path_overlap,
        "title_overlap": title_overlap,
        "anchor_hits": anchor_hits,
        "baseline_bonus": baseline_bonus,
        "score": round(score, 4),
    }


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def select_docs(
    *,
    question: dict[str, Any],
    baseline_ids: list[str],
    candidate_ids: list[str],
    title_cache: TitleCache,
    args: argparse.Namespace,
) -> tuple[list[str], list[dict[str, Any]]]:
    baseline_set = set(baseline_ids[: args.limit])
    scored: list[tuple[float, int, str, dict[str, Any]]] = []
    for rank, doc_id in enumerate(candidate_ids[: args.candidate_rank_limit], 1):
        value = score_candidate(
            question=question,
            doc_id=doc_id,
            rank=rank,
            baseline_ids=baseline_set,
            title_cache=title_cache,
        )
        if value is None:
            continue
        score, features = value
        scored.append((score, rank, doc_id, features))
    scored.sort(key=lambda item: (-item[0], item[1], item[2]))

    selected: list[str] = []
    seen: set[str] = set()
    for doc_id in baseline_ids[: min(args.protect_baseline_prefix, args.limit)]:
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)
    for _score, _rank, doc_id, _features in scored:
        if len(selected) >= args.limit:
            break
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)
    for doc_id in baseline_ids:
        if len(selected) >= args.limit:
            break
        if doc_id not in seen:
            selected.append(doc_id)
            seen.add(doc_id)
    return selected[: args.limit], [
        {"doc_id": doc_id, **features}
        for _score, _rank, doc_id, features in scored[: args.diagnostics_top_k]
    ]


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    candidates = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidates")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    title_cache = TitleCache(uuid_index, args.sources_dir)

    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    changed_rows = 0
    routed_rows = 0
    diagnostics: dict[str, Any] = {}

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        output = dict(base_row)
        if is_routed(question):
            routed_rows += 1
            selected, top_features = select_docs(
                question=question,
                baseline_ids=baseline_ids,
                candidate_ids=doc_ids(candidates.get(qid)),
                title_cache=title_cache,
                args=args,
            )
            if selected != baseline_ids:
                changed_rows += 1
            output["document_ids"] = selected
            output["route"] = {
                "policy": args.policy_name,
                "enabled": True,
                "source": "confluence_completeness_selector",
                "candidate_rank_limit": args.candidate_rank_limit,
                "protect_baseline_prefix": args.protect_baseline_prefix,
            }
            if args.diagnostics_top_k:
                diagnostics[qid] = {
                    "top_features": top_features,
                    "selected": selected,
                    "baseline": baseline_ids,
                }
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {
                "policy": args.policy_name,
                "enabled": False,
                "source": "confluence_completeness_selector",
            }
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.confluence_completeness_selector.v1",
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "output": str(args.output),
        "candidate_rank_limit": args.candidate_rank_limit,
        "protect_baseline_prefix": args.protect_baseline_prefix,
        "changed_rows": changed_rows,
        "routed_rows": routed_rows,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2)
        if recall_values
        else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "diagnostics": diagnostics,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware selection.",
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
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="confluence_completeness_selector_v1")
    parser.add_argument("--candidate-rank-limit", type=int, default=50)
    parser.add_argument("--protect-baseline-prefix", type=int, default=2)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--diagnostics-top-k", type=int, default=0)
    args = parser.parse_args()
    if args.candidate_rank_limit <= 0:
        parser.error("--candidate-rank-limit must be positive")
    if args.protect_baseline_prefix < 0:
        parser.error("--protect-baseline-prefix must be non-negative")
    if args.limit <= 0:
        parser.error("--limit must be positive")
    if args.protect_baseline_prefix > args.limit:
        parser.error("--protect-baseline-prefix cannot exceed --limit")
    if args.diagnostics_top_k < 0:
        parser.error("--diagnostics-top-k must be non-negative")
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
                "output": report["output"],
                "routed_rows": report["routed_rows"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
