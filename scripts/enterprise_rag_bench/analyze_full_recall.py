#!/usr/bin/env python3
"""Build a per-question EnterpriseRAG failure ledger."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

from question_decomposition import evidence_units, tokens


def read_json(path: Path | None) -> Any:
    if path is None or not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
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


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Full Failure Ledger Summary",
        "",
        f"- questions: `{report['questions']}`",
        f"- selected_retrieval_file: `{report.get('selected_retrieval_file')}`",
        f"- candidate_retrieval_file: `{report.get('candidate_retrieval_file')}`",
        "",
        "## Buckets",
        "",
        "| Bucket | Count |",
        "| --- | ---: |",
    ]
    for bucket, count in sorted(report["bucket_counts"].items()):
        lines.append(f"| `{bucket}` | {count} |")
    lines.extend(
        [
            "",
            "## Question Types",
            "",
            "| Type | Count | Candidate Missing | Rerank Miss | Selected Hit |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for question_type, stats in sorted(report["question_type_stats"].items()):
        lines.append(
            "| {question_type} | {count} | {candidate_missing} | {rerank_miss} | "
            "{selected_hit} |".format(question_type=question_type, **stats)
        )
    lines.extend(
        [
            "",
            "## First Actions",
            "",
            "- `gold_not_in_candidates`: improve first-stage candidate generation.",
            "- `gold_in_candidates_not_selected`: improve rerank/evidence selection.",
            "- `answer_missing_gold_facts`: improve windowing/digest/answer synthesis.",
        ]
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in indexed:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        indexed[qid] = row
    return indexed


def metric_rows_by_id(metrics: dict[str, Any]) -> dict[str, dict[str, Any]]:
    rows = metrics.get("questions")
    if not isinstance(rows, list):
        return {}
    return rows_by_id([row for row in rows if isinstance(row, dict)], "metrics")


def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def expected_docs(question: dict[str, Any]) -> list[str]:
    return [str(item) for item in question.get("expected_doc_ids", []) if str(item)]


def source_type_for_doc(doc_id: str, uuid_index: dict[str, str]) -> str:
    path = uuid_index.get(doc_id, "")
    if not path:
        return "unknown"
    return path.split("/", 1)[0]


def positions(expected: set[str], candidates: list[str]) -> dict[str, int | None]:
    index = {doc_id: rank for rank, doc_id in enumerate(candidates, 1)}
    return {doc_id: index.get(doc_id) for doc_id in sorted(expected)}


def first_rank(position_map: dict[str, int | None]) -> int | None:
    ranks = [rank for rank in position_map.values() if rank is not None]
    return min(ranks) if ranks else None


def depth_bucket(rank: int | None) -> str:
    if rank is None:
        return "gold_not_in_candidates"
    if rank <= 10:
        return "gold_in_top10"
    if rank <= 50:
        return "gold_in_top50_not_top10"
    if rank <= 100:
        return "gold_in_top100_not_top50"
    if rank <= 500:
        return "gold_in_top500_not_top100"
    if rank <= 1000:
        return "gold_in_top1000_not_top500"
    return "gold_after_top1000"


def lexical_terms(question: str) -> list[str]:
    return sorted(set(tokens(question)), key=lambda item: (-len(item), item))[:24]


def candidate_bucket(
    *,
    expected: set[str],
    selected: set[str],
    candidates: set[str],
    metric: dict[str, Any],
) -> list[str]:
    buckets: list[str] = []
    if not expected:
        buckets.append("no_expected_docs")
    elif not expected & candidates:
        buckets.append("gold_not_in_candidates")
    elif not expected & selected:
        buckets.append("gold_in_candidates_not_selected")
    else:
        buckets.append("selected_has_gold")

    recall = metric.get("document_recall_pct")
    correct = metric.get("answer_correct")
    completeness = float(metric.get("completeness_pct") or 0.0)
    if recall is not None and float(recall) > 0 and correct is False:
        buckets.append("answer_missing_gold_facts")
    elif recall is not None and float(recall) > 0 and completeness < 100.0:
        buckets.append("answer_incomplete")
    return buckets


def source_bucket(
    expected_source_types: set[str],
    selected_doc_ids: list[str],
    uuid_index: dict[str, str],
) -> str | None:
    if not expected_source_types or not selected_doc_ids or not uuid_index:
        return None
    selected_sources = {source_type_for_doc(doc_id, uuid_index) for doc_id in selected_doc_ids}
    if expected_source_types & selected_sources:
        return None
    return "wrong_source_type"


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    selected_rows = rows_by_id(read_jsonl(args.selected_retrieval_file), "selected retrieval")
    candidate_rows = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidate retrieval")
    metrics = metric_rows_by_id(read_json(args.metrics_file))
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        uuid_index = {}

    ledger_rows: list[dict[str, Any]] = []
    bucket_counts: dict[str, int] = defaultdict(int)
    type_stats: dict[str, dict[str, int]] = defaultdict(
        lambda: {"count": 0, "candidate_missing": 0, "rerank_miss": 0, "selected_hit": 0}
    )

    for qid, question in sorted(questions.items()):
        selected = doc_ids(selected_rows.get(qid))
        candidates = doc_ids(candidate_rows.get(qid))
        expected = set(expected_docs(question))
        selected_set = set(selected)
        candidate_set = set(candidates)
        candidate_positions = positions(expected, candidates)
        selected_positions = positions(expected, selected)
        candidate_first_rank = first_rank(candidate_positions)
        selected_first_rank = first_rank(selected_positions)
        metric = metrics.get(qid, {})

        buckets = candidate_bucket(
            expected=expected,
            selected=selected_set,
            candidates=candidate_set,
            metric=metric,
        )
        source_miss = source_bucket(
            {str(item) for item in question.get("source_types", [])},
            selected,
            uuid_index,
        )
        if source_miss:
            buckets.append(source_miss)
        qtype = str(question.get("question_type") or "unknown")
        if qtype == "semantic" and "gold_not_in_candidates" in buckets:
            buckets.append("semantic_miss")
        if qtype == "project_related" and "gold_not_in_candidates" in buckets:
            buckets.append("project_chain_miss")
        for bucket in buckets:
            bucket_counts[bucket] += 1

        stats = type_stats[qtype]
        stats["count"] += 1
        if not expected & candidate_set:
            stats["candidate_missing"] += 1
        elif not expected & selected_set:
            stats["rerank_miss"] += 1
        else:
            stats["selected_hit"] += 1

        units = evidence_units(str(question.get("question", "")))
        ledger_rows.append(
            {
                "question_id": qid,
                "question_type": qtype,
                "question": question.get("question", ""),
                "source_types": question.get("source_types", []),
                "query_terms": lexical_terms(str(question.get("question", ""))),
                "evidence_units": [
                    {"id": unit["id"], "kind": unit["kind"], "text": unit["text"]}
                    for unit in units
                ],
                "expected_doc_ids": sorted(expected),
                "selected_doc_ids": selected,
                "candidate_doc_count": len(candidates),
                "missing_gold_doc_ids": sorted(expected - candidate_set),
                "unselected_gold_doc_ids": sorted((expected & candidate_set) - selected_set),
                "invalid_extra_docs": metric.get("invalid_extra_docs"),
                "document_recall_pct": metric.get("document_recall_pct"),
                "answer_correct": metric.get("answer_correct"),
                "completeness_pct": metric.get("completeness_pct"),
                "candidate_gold_positions": candidate_positions,
                "selected_gold_positions": selected_positions,
                "candidate_first_gold_rank": candidate_first_rank,
                "selected_first_gold_rank": selected_first_rank,
                "candidate_depth_bucket": depth_bucket(candidate_first_rank),
                "failure_buckets": buckets,
            }
        )

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.full_failure_ledger.v1",
        "questions_file": str(args.questions_file),
        "selected_retrieval_file": str(args.selected_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "metrics_file": str(args.metrics_file) if args.metrics_file else None,
        "uuid_index": str(args.uuid_index) if args.uuid_index else None,
        "questions": len(ledger_rows),
        "ledger_file": str(args.output_jsonl),
        "bucket_counts": dict(sorted(bucket_counts.items())),
        "question_type_stats": dict(sorted(type_stats.items())),
    }
    write_jsonl(args.output_jsonl, ledger_rows)
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--selected-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--metrics-file", type=Path)
    parser.add_argument("--uuid-index", type=Path)
    return parser.parse_args()


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "questions": report["questions"],
                "bucket_counts": report["bucket_counts"],
                "ledger_file": report["ledger_file"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
