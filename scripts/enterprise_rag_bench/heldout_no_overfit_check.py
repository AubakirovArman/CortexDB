#!/usr/bin/env python3
"""Check EnterpriseRAG retrieval transfer from tuning split to held-out split."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

from official_clean import assert_clean_retrieval, clean_question, read_jsonl, write_json, write_jsonl


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id {qid}")
        values[qid] = row
    return values


def expected_doc_ids(question: dict[str, Any]) -> list[str]:
    values: list[str] = []
    seen: set[str] = set()
    for item in question.get("expected_doc_ids", []):
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            values.append(doc_id)
            seen.add(doc_id)
    return values


def retrieved_doc_ids(row: dict[str, Any] | None, limit: int) -> list[str]:
    if row is None:
        return []
    values: list[str] = []
    seen: set[str] = set()
    for item in row.get("document_ids", []):
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            values.append(doc_id)
            seen.add(doc_id)
        if len(values) >= limit:
            break
    return values


def stable_question_ids(questions: dict[str, dict[str, Any]], seed: str, heldout_size: int) -> tuple[list[str], list[str]]:
    if heldout_size <= 0:
        raise ValueError("heldout_size must be positive")
    if heldout_size >= len(questions):
        raise ValueError("heldout_size must be smaller than question count")
    ranked = sorted(
        questions,
        key=lambda qid: hashlib.sha256(f"{seed}:{qid}".encode("utf-8")).hexdigest(),
    )
    heldout = sorted(ranked[:heldout_size])
    tuning = sorted(qid for qid in questions if qid not in set(heldout))
    return tuning, heldout


def reciprocal_rank(expected: set[str], retrieved: list[str]) -> float:
    for index, doc_id in enumerate(retrieved, 1):
        if doc_id in expected:
            return 1.0 / index
    return 0.0


def dcg(expected: set[str], retrieved: list[str]) -> float:
    return sum(
        1.0 / math.log2(index + 1)
        for index, doc_id in enumerate(retrieved, 1)
        if doc_id in expected
    )


def ndcg(expected: set[str], retrieved: list[str]) -> float:
    if not expected:
        return 0.0
    ideal_len = min(len(expected), len(retrieved))
    if ideal_len == 0:
        return 0.0
    ideal = sum(1.0 / math.log2(index + 1) for index in range(1, ideal_len + 1))
    return dcg(expected, retrieved) / ideal if ideal else 0.0


def mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def round2(value: float) -> float:
    return round(value, 2)


def score_split(
    question_ids: list[str],
    questions: dict[str, dict[str, Any]],
    retrieval: dict[str, dict[str, Any]],
    top_k: int,
) -> dict[str, Any]:
    recall_values: list[float] = []
    precision_values: list[float] = []
    invalid_values: list[float] = []
    mrr_values: list[float] = []
    ndcg_values: list[float] = []
    missing_retrieval_rows: list[str] = []
    hit_questions = 0
    full_recall_questions = 0
    answerable_questions = 0

    for qid in question_ids:
        question = questions[qid]
        expected = expected_doc_ids(question)
        retrieved = retrieved_doc_ids(retrieval.get(qid), top_k)
        if qid not in retrieval:
            missing_retrieval_rows.append(qid)
        expected_set = set(expected)
        hits = len(expected_set & set(retrieved))
        invalid_values.append(float(len([doc_id for doc_id in retrieved if doc_id not in expected_set])))
        if not expected:
            continue
        answerable_questions += 1
        recall = hits / len(expected_set) * 100.0
        precision = hits / len(retrieved) * 100.0 if retrieved else 0.0
        recall_values.append(recall)
        precision_values.append(precision)
        mrr_values.append(reciprocal_rank(expected_set, retrieved))
        ndcg_values.append(ndcg(expected_set, retrieved))
        if hits > 0:
            hit_questions += 1
        if hits == len(expected_set):
            full_recall_questions += 1

    return {
        "questions": len(question_ids),
        "answerable_questions": answerable_questions,
        "average_recall_pct": round2(mean(recall_values)),
        "average_precision_pct": round2(mean(precision_values)),
        "average_invalid_extra_docs": round2(mean(invalid_values)),
        "hit_questions": hit_questions,
        "full_recall_questions": full_recall_questions,
        "mrr": round2(mean(mrr_values)),
        "ndcg": round2(mean(ndcg_values)),
        "missing_retrieval_rows": missing_retrieval_rows,
    }


def write_split_files(
    output_root: Path,
    questions: dict[str, dict[str, Any]],
    tuning_ids: list[str],
    heldout_ids: list[str],
) -> dict[str, str]:
    output_root.mkdir(parents=True, exist_ok=True)
    tuning_ids_path = output_root / "tuning_question_ids.json"
    heldout_ids_path = output_root / "heldout_question_ids.json"
    tuning_clean_path = output_root / "tuning.questions.clean.jsonl"
    heldout_clean_path = output_root / "heldout.questions.clean.jsonl"

    tuning_ids_path.write_text(json.dumps(tuning_ids, indent=2) + "\n", encoding="utf-8")
    heldout_ids_path.write_text(json.dumps(heldout_ids, indent=2) + "\n", encoding="utf-8")
    write_jsonl(tuning_clean_path, [clean_question(questions[qid]) for qid in tuning_ids])
    write_jsonl(heldout_clean_path, [clean_question(questions[qid]) for qid in heldout_ids])
    return {
        "tuning_question_ids": str(tuning_ids_path),
        "heldout_question_ids": str(heldout_ids_path),
        "tuning_clean_questions": str(tuning_clean_path),
        "heldout_clean_questions": str(heldout_clean_path),
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Held-out No-overfit Check",
        "",
        f"- status: `{report['status']}`",
        f"- seed: `{report['seed']}`",
        f"- top_k: `{report['top_k']}`",
        "",
        "## Metrics",
        "",
        "| Split | Qs | Answerable | Recall | Precision | Invalid | Hit | Full | MRR | nDCG |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for split in ("tuning", "heldout"):
        row = report["metrics"][split]
        lines.append(
            f"| `{split}` | {row['questions']} | {row['answerable_questions']} | "
            f"{row['average_recall_pct']} | {row['average_precision_pct']} | "
            f"{row['average_invalid_extra_docs']} | {row['hit_questions']} | "
            f"{row['full_recall_questions']} | {row['mrr']} | {row['ndcg']} |"
        )
    delta = report["metrics"]["delta"]
    lines.extend(
        [
            "",
            "## Delta",
            "",
            f"- recall_delta_pct: `{delta['average_recall_delta_pct']}`",
            f"- absolute_recall_delta_pct: `{delta['absolute_average_recall_delta_pct']}`",
            f"- allowed_absolute_recall_delta_pct: `{report['thresholds']['max_absolute_recall_delta_pct']}`",
        ]
    )
    if report["errors"]:
        lines.extend(["", "## Errors", ""])
        lines.extend(f"- {error}" for error in report["errors"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    retrieval_rows = read_jsonl(args.retrieval_file)
    assert_clean_retrieval(retrieval_rows)
    retrieval = rows_by_id(retrieval_rows, "retrieval")
    tuning_ids, heldout_ids = stable_question_ids(questions, args.seed, args.heldout_size)

    split_files = write_split_files(args.output_root, questions, tuning_ids, heldout_ids)
    tuning = score_split(tuning_ids, questions, retrieval, args.top_k)
    heldout = score_split(heldout_ids, questions, retrieval, args.top_k)
    recall_delta = round2(heldout["average_recall_pct"] - tuning["average_recall_pct"])
    abs_recall_delta = abs(recall_delta)

    errors: list[str] = []
    missing_count = len(tuning["missing_retrieval_rows"]) + len(heldout["missing_retrieval_rows"])
    if missing_count:
        errors.append(f"missing retrieval rows: {missing_count}")
    if abs_recall_delta > args.max_absolute_recall_delta_pct:
        errors.append(
            "absolute average recall delta "
            f"{abs_recall_delta} > {args.max_absolute_recall_delta_pct}"
        )

    return {
        "schema_version": "cortexdb.enterprise_rag_bench.heldout_no_overfit.v1",
        "status": "passed" if not errors else "failed",
        "questions_file": str(args.questions_file),
        "retrieval_file": str(args.retrieval_file),
        "seed": args.seed,
        "top_k": args.top_k,
        "split_files": split_files,
        "thresholds": {
            "max_absolute_recall_delta_pct": args.max_absolute_recall_delta_pct,
        },
        "metrics": {
            "tuning": tuning,
            "heldout": heldout,
            "delta": {
                "average_recall_delta_pct": recall_delta,
                "absolute_average_recall_delta_pct": round2(abs_recall_delta),
                "average_precision_delta_pct": round2(
                    heldout["average_precision_pct"] - tuning["average_precision_pct"]
                ),
                "average_invalid_extra_delta": round2(
                    heldout["average_invalid_extra_docs"] - tuning["average_invalid_extra_docs"]
                ),
                "mrr_delta": round2(heldout["mrr"] - tuning["mrr"]),
                "ndcg_delta": round2(heldout["ndcg"] - tuning["ndcg"]),
            },
        },
        "errors": errors,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--heldout-size", type=int, default=100)
    parser.add_argument("--seed", default="cortexdb-erb-heldout-v1")
    parser.add_argument("--max-absolute-recall-delta-pct", type=float, default=2.0)
    args = parser.parse_args()
    if args.top_k <= 0:
        parser.error("--top-k must be positive")
    return args


def main() -> int:
    args = parse_args()
    report = build_report(args)
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    print(
        json.dumps(
            {
                "status": report["status"],
                "tuning_recall_pct": report["metrics"]["tuning"]["average_recall_pct"],
                "heldout_recall_pct": report["metrics"]["heldout"]["average_recall_pct"],
                "absolute_recall_delta_pct": report["metrics"]["delta"][
                    "absolute_average_recall_delta_pct"
                ],
                "output": str(args.report),
            },
            sort_keys=True,
        )
    )
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
