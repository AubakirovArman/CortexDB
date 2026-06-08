#!/usr/bin/env python3
"""Classify why EnterpriseRAG gold documents are missing from final top-k.

This is a local retrieval diagnostic. It does not call an LLM and it does not
use answer text to retrieve documents. The goal is to make every missing gold
document actionable: candidate generation miss, rank-depth miss, fusion loss,
source routing issue, or likely near-duplicate confusion.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from question_decomposition import tokens


def read_json(path: Path | None) -> Any:
    if path is None or not path.exists():
        return {}
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


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Gold Missing Reason Classifier",
        "",
        f"- questions: `{report['questions']}`",
        f"- final_retrieval_file: `{report['final_retrieval_file']}`",
        f"- candidate_retrieval_file: `{report['candidate_retrieval_file']}`",
        f"- missing_gold_docs: `{report['missing_gold_docs']}`",
        "",
        "## Reason Buckets",
        "",
        "| Reason | Missing Gold Docs |",
        "| --- | ---: |",
    ]
    for reason, count in sorted(report["reason_counts"].items()):
        lines.append(f"| `{reason}` | {count} |")

    lines.extend(
        [
            "",
            "## By Question Type",
            "",
            "| Question Type | Missing Gold Docs | Top Reason |",
            "| --- | ---: | --- |",
        ]
    )
    for question_type, stats in sorted(report["question_type_stats"].items()):
        lines.append(
            f"| `{question_type}` | {stats['missing_gold_docs']} | "
            f"`{stats['top_reason']}` |"
        )

    lines.extend(
        [
            "",
            "## How To Use",
            "",
            "- `not_in_top1000`: improve first-stage discovery.",
            "- `in_top500_not_top50` / `in_top50_not_top10`: improve reranking and top-k composition.",
            "- `lost_by_embedding_rerank` / `lost_by_rrf`: inspect fusion weights.",
            "- `near_duplicate_confusion`: add diversity or thread/variant expansion.",
            "- `high_level_no_single_doc`: route to high-level coverage mode.",
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


def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def expected_docs(question: dict[str, Any]) -> list[str]:
    return [str(item) for item in question.get("expected_doc_ids", []) if str(item)]


def source_type(rel_path: str) -> str:
    return rel_path.split("/", 1)[0] if rel_path else "unknown"


def path_terms(rel_path: str) -> set[str]:
    parts = rel_path.replace(".json", " ").replace("/", " ").replace("-", " ").replace("_", " ")
    return set(tokens(parts))


def rank_of(doc_id: str, docs: list[str]) -> int | None:
    try:
        return docs.index(doc_id) + 1
    except ValueError:
        return None


def rank_bucket(rank: int | None) -> str:
    if rank is None:
        return "not_in_top1000"
    if rank <= 10:
        return "in_top10_but_missing_final"
    if rank <= 50:
        return "in_top50_not_top10"
    if rank <= 100:
        return "in_top100_not_top50"
    if rank <= 500:
        return "in_top500_not_top100"
    if rank <= 1000:
        return "in_top1000_not_top500"
    return "after_top1000"


def near_duplicate_confusion(
    *,
    gold_doc_id: str,
    final_docs: list[str],
    uuid_index: dict[str, str],
) -> bool:
    gold_path = uuid_index.get(gold_doc_id, "")
    gold_source = source_type(gold_path)
    gold_terms = path_terms(gold_path)
    if not gold_terms:
        return False
    for doc_id in final_docs:
        candidate_path = uuid_index.get(doc_id, "")
        if source_type(candidate_path) != gold_source:
            continue
        candidate_terms = path_terms(candidate_path)
        if not candidate_terms:
            continue
        overlap = len(gold_terms & candidate_terms)
        union = len(gold_terms | candidate_terms)
        if union and overlap / union >= 0.35:
            return True
    return False


def source_filtered(
    *,
    question: dict[str, Any],
    gold_doc_id: str,
    final_docs: list[str],
    uuid_index: dict[str, str],
) -> bool:
    expected_sources = {str(item) for item in question.get("source_types", []) if str(item)}
    if not expected_sources:
        return False
    gold_source = source_type(uuid_index.get(gold_doc_id, ""))
    if gold_source not in expected_sources:
        return True
    final_sources = {source_type(uuid_index.get(doc_id, "")) for doc_id in final_docs}
    return gold_source not in final_sources


def classify_missing_doc(
    *,
    question: dict[str, Any],
    gold_doc_id: str,
    final_docs: list[str],
    candidate_docs: list[str],
    named_rankings: dict[str, list[str]],
    uuid_index: dict[str, str],
) -> tuple[str, dict[str, Any]]:
    ranks = {name: rank_of(gold_doc_id, docs) for name, docs in named_rankings.items()}
    candidate_rank = rank_of(gold_doc_id, candidate_docs)
    qtype = str(question.get("question_type") or "unknown")

    if qtype == "high_level":
        return "high_level_no_single_doc", {"ranks": ranks, "candidate_rank": candidate_rank}
    if source_filtered(question=question, gold_doc_id=gold_doc_id, final_docs=final_docs, uuid_index=uuid_index):
        return "filtered_by_source", {"ranks": ranks, "candidate_rank": candidate_rank}
    if near_duplicate_confusion(gold_doc_id=gold_doc_id, final_docs=final_docs, uuid_index=uuid_index):
        return "near_duplicate_confusion", {"ranks": ranks, "candidate_rank": candidate_rank}

    if candidate_rank is None:
        best_named_rank = min((rank for rank in ranks.values() if rank is not None), default=None)
        if best_named_rank is None:
            return "not_in_top1000", {"ranks": ranks, "candidate_rank": candidate_rank}
        return rank_bucket(best_named_rank), {"ranks": ranks, "candidate_rank": candidate_rank}

    dense_rank = ranks.get("dense")
    rrf_rank = ranks.get("rrf")
    if candidate_rank <= 50 and dense_rank is None:
        return "lost_by_embedding_rerank", {"ranks": ranks, "candidate_rank": candidate_rank}
    if candidate_rank <= 50 and rrf_rank is None:
        return "lost_by_rrf", {"ranks": ranks, "candidate_rank": candidate_rank}
    return rank_bucket(candidate_rank), {"ranks": ranks, "candidate_rank": candidate_rank}


def load_named_rankings(paths: list[str], default_name: str) -> dict[str, dict[str, list[str]]]:
    values: dict[str, dict[str, list[str]]] = {}
    for index, spec in enumerate(paths, 1):
        if "=" in spec:
            name, raw_path = spec.split("=", 1)
        else:
            name, raw_path = f"{default_name}_{index}", spec
        rows = rows_by_id(read_jsonl(Path(raw_path)), name)
        values[name] = {qid: doc_ids(row) for qid, row in rows.items()}
    return values


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    final_rows = rows_by_id(read_jsonl(args.final_retrieval_file), "final retrieval")
    candidate_rows = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidate retrieval")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        uuid_index = {}
    named_sources = load_named_rankings(args.compare_retrieval_file, "compare")

    detail_rows: list[dict[str, Any]] = []
    reason_counts: Counter[str] = Counter()
    type_reason_counts: dict[str, Counter[str]] = defaultdict(Counter)
    missing_questions: set[str] = set()

    for qid, question in sorted(questions.items()):
        final_docs = doc_ids(final_rows.get(qid))
        candidate_docs = doc_ids(candidate_rows.get(qid))
        final_set = set(final_docs)
        expected = expected_docs(question)
        qtype = str(question.get("question_type") or "unknown")
        named_rankings = {
            name: docs_by_qid.get(qid, [])
            for name, docs_by_qid in named_sources.items()
        }
        named_rankings["candidate"] = candidate_docs
        named_rankings["final"] = final_docs

        for gold_doc_id in expected:
            if gold_doc_id in final_set:
                continue
            reason, extra = classify_missing_doc(
                question=question,
                gold_doc_id=gold_doc_id,
                final_docs=final_docs,
                candidate_docs=candidate_docs,
                named_rankings=named_rankings,
                uuid_index=uuid_index,
            )
            reason_counts[reason] += 1
            type_reason_counts[qtype][reason] += 1
            missing_questions.add(qid)
            detail_rows.append(
                {
                    "question_id": qid,
                    "question_type": qtype,
                    "question": question.get("question", ""),
                    "gold_doc_id": gold_doc_id,
                    "gold_path": uuid_index.get(gold_doc_id, ""),
                    "reason": reason,
                    "candidate_rank": extra["candidate_rank"],
                    "ranks": extra["ranks"],
                    "final_doc_ids": final_docs,
                    "final_paths": [uuid_index.get(doc_id, "") for doc_id in final_docs],
                    "source_types": question.get("source_types", []),
                }
            )

    question_type_stats: dict[str, dict[str, Any]] = {}
    for qtype, counts in sorted(type_reason_counts.items()):
        top_reason, _count = counts.most_common(1)[0]
        question_type_stats[qtype] = {
            "missing_gold_docs": sum(counts.values()),
            "reasons": dict(sorted(counts.items())),
            "top_reason": top_reason,
        }

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.gold_missing_reasons.v1",
        "questions": len(questions),
        "questions_with_missing_gold": len(missing_questions),
        "missing_gold_docs": len(detail_rows),
        "questions_file": str(args.questions_file),
        "final_retrieval_file": str(args.final_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "compare_retrieval_files": args.compare_retrieval_file,
        "uuid_index": str(args.uuid_index),
        "details_file": str(args.output_jsonl),
        "report_file": str(args.report),
        "reason_counts": dict(sorted(reason_counts.items())),
        "question_type_stats": question_type_stats,
    }
    write_jsonl(args.output_jsonl, detail_rows)
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--final-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument(
        "--compare-retrieval-file",
        action="append",
        default=[],
        help="Optional NAME=PATH retrieval file used to identify where a gold doc was lost.",
    )
    return parser.parse_args()


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "missing_gold_docs": report["missing_gold_docs"],
                "questions_with_missing_gold": report["questions_with_missing_gold"],
                "reason_counts": report["reason_counts"],
                "report": report["report_file"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
