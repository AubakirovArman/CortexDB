#!/usr/bin/env python3
"""Audit where expected EnterpriseRAG documents appear in a candidate list."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


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


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Candidate Depth Audit",
        "",
        f"- questions: `{report['questions']}`",
        f"- retrieval_file: `{report['retrieval_file']}`",
        "",
        "| Depth | Hit Questions | Question Hit % | Average Gold Recall % | Full Recall Questions |",
        "| ---: | ---: | ---: | ---: | ---: |",
    ]
    for depth, stats in report["depth_stats"].items():
        lines.append(
            "| {depth} | {hit_questions} | {question_hit_pct} | {average_recall_pct} | "
            "{full_recall_questions} |".format(depth=depth, **stats)
        )
    lines.extend(
        [
            "",
            "## Buckets",
            "",
            "| Bucket | Count |",
            "| --- | ---: |",
        ]
    )
    for bucket, count in sorted(report["bucket_counts"].items()):
        lines.append(f"| `{bucket}` | {count} |")
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


def doc_ids(row: dict[str, Any]) -> list[str]:
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def expected_docs(question: dict[str, Any]) -> list[str]:
    return [str(item) for item in question.get("expected_doc_ids", []) if str(item)]


def recall_at(expected: set[str], candidates: list[str], depth: int) -> float | None:
    if not expected:
        return None
    retrieved = set(candidates[:depth])
    return round(len(expected & retrieved) / len(expected) * 100.0, 2)


def first_gold_rank(expected: set[str], candidates: list[str]) -> int | None:
    for index, doc_id in enumerate(candidates, 1):
        if doc_id in expected:
            return index
    return None


def bucket_for_rank(rank: int | None) -> str:
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


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def run(args: argparse.Namespace) -> dict[str, Any]:
    depths = sorted({int(item) for item in args.depths.split(",") if item.strip()})
    if not depths or min(depths) <= 0:
        raise ValueError("--depths must contain positive integers")

    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval_rows = rows_by_id(read_jsonl(args.retrieval_file), "retrieval")

    detail_rows: list[dict[str, Any]] = []
    bucket_counts: dict[str, int] = defaultdict(int)
    per_depth_recalls: dict[int, list[float]] = {depth: [] for depth in depths}
    per_depth_hits: dict[int, int] = {depth: 0 for depth in depths}
    per_depth_full: dict[int, int] = {depth: 0 for depth in depths}
    first_ranks: list[int] = []
    missing_questions: list[str] = []

    for qid, question in sorted(questions.items()):
        row = retrieval_rows.get(qid, {"document_ids": []})
        candidates = doc_ids(row)
        expected = set(expected_docs(question))
        rank = first_gold_rank(expected, candidates)
        if rank is None:
            missing_questions.append(qid)
        else:
            first_ranks.append(rank)
        bucket = bucket_for_rank(rank)
        bucket_counts[bucket] += 1
        depth_recalls: dict[str, float | None] = {}
        for depth in depths:
            recall = recall_at(expected, candidates, depth)
            depth_recalls[str(depth)] = recall
            if recall is not None:
                per_depth_recalls[depth].append(recall)
                if recall > 0:
                    per_depth_hits[depth] += 1
                if recall == 100.0:
                    per_depth_full[depth] += 1
        detail_rows.append(
            {
                "question_id": qid,
                "question_type": question.get("question_type"),
                "source_types": question.get("source_types", []),
                "expected_doc_ids": sorted(expected),
                "expected_doc_count": len(expected),
                "candidate_count": len(candidates),
                "first_gold_rank": rank,
                "bucket": bucket,
                "recall_at": depth_recalls,
                "question": question.get("question", ""),
            }
        )

    depth_stats = {}
    question_count = len(detail_rows)
    for depth in depths:
        hit_count = per_depth_hits[depth]
        depth_stats[str(depth)] = {
            "hit_questions": hit_count,
            "question_hit_pct": round(hit_count / question_count * 100.0, 2) if question_count else 0.0,
            "average_recall_pct": mean(per_depth_recalls[depth]),
            "full_recall_questions": per_depth_full[depth],
        }

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.candidate_depth_audit.v1",
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "report_file": str(args.report),
        "questions": question_count,
        "depths": depths,
        "depth_stats": depth_stats,
        "bucket_counts": dict(sorted(bucket_counts.items())),
        "missing_questions": missing_questions,
        "first_gold_rank_stats": {
            "available_questions": len(first_ranks),
            "missing_questions": len(missing_questions),
            "average_first_gold_rank": round(sum(first_ranks) / len(first_ranks), 2) if first_ranks else None,
            "max_first_gold_rank": max(first_ranks) if first_ranks else None,
        },
        "details_file": str(args.output_jsonl),
    }
    write_jsonl(args.output_jsonl, detail_rows)
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--depths", default="10,50,100,500,1000")
    return parser.parse_args()


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "questions": report["questions"],
                "depth_stats": report["depth_stats"],
                "report": report["report_file"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
