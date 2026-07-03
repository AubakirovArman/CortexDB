#!/usr/bin/env python3
"""F2.3: package the official LongMemEval per-type score.

Aggregates a LongMemEval run into a per-question-type draft snapshot: QA accuracy
(from the evaluator output) plus retrieval recall_all@10 / ndcg_any@10 (from the
retrieval log + reference), emitted as a schema-valid
`cortexdb.longmemeval.per_type_report.v1` document. It is the packaging half of
the F2/F3 LongMemEval cluster (the regression *gate* is F3.1); it reuses the same
per-type aggregation the official analysis does.

Asserts (per the master plan):
  - every LongMemEval dataset question type is present in the report;
  - the recomputed overall QA accuracy matches the evaluator's reported overall
    (guards against a packaging/aggregation drift).

Entry points:
  - `--self-test` (offline): package a synthetic run and verify the per-type
    aggregation, the all-types-present assertion, and the overall-matches
    assertion — proving the packaging logic without a metered run.
  - REAL: `--reference --retrieval-log --evaluator-output [--reported-overall]`
    package an official run into `--output`.

Dependency-free (stdlib only); deterministic; no network, no wall clock, no LLM.
"""

from __future__ import annotations

import argparse
import json
import math
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
SCHEMA_VERSION = "cortexdb.longmemeval.per_type_report.v1"
K = 10
ROUND = 6
OVERALL_MATCH_TOL = 1e-6

# The six official LongMemEval question types (the report must cover them all).
LONGMEMEVAL_TYPES = (
    "single-session-user",
    "single-session-assistant",
    "single-session-preference",
    "multi-session",
    "temporal-reasoning",
    "knowledge-update",
)


def recall_all_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    if not gold:
        return 0.0
    topk = ranked[:k]
    return sum(1 for g in gold if g in topk) / len(gold)


def ndcg_any_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    if not gold:
        return 0.0
    dcg = sum(1.0 / math.log2(i + 1) for i, doc in enumerate(ranked[:k], 1) if doc in gold)
    ideal = min(len(gold), k)
    idcg = sum(1.0 / math.log2(i + 1) for i in range(1, ideal + 1))
    return dcg / idcg if idcg else 0.0


def package(reference: list[dict], retrieval_log: list[dict], evaluator: dict) -> dict:
    """reference: [{question_id, question_type, gold_ids}]; retrieval_log:
    [{question_id, retrieved_ids}]; evaluator: {question_id: is_correct(bool)}.
    Returns the per-type + overall draft snapshot."""
    ranked_by_q = {row["question_id"]: row["retrieved_ids"] for row in retrieval_log}
    by_type: dict[str, list[dict]] = {}
    allrows: list[dict] = []
    for ref in reference:
        qid = ref["question_id"]
        ranked = ranked_by_q.get(qid, [])
        gold = set(ref["gold_ids"])
        row = {
            "correct": 1.0 if evaluator.get(qid) else 0.0,
            "recall_all@10": recall_all_at_k(ranked, gold, K),
            "ndcg_any@10": ndcg_any_at_k(ranked, gold, K),
        }
        by_type.setdefault(ref["question_type"], []).append(row)
        allrows.append(row)

    def agg(rows: list[dict]) -> dict:
        n = len(rows)
        return {
            "n": n,
            "qa_accuracy": round(sum(r["correct"] for r in rows) / n, ROUND) if n else 0.0,
            "recall_all@10": round(sum(r["recall_all@10"] for r in rows) / n, ROUND) if n else 0.0,
            "ndcg_any@10": round(sum(r["ndcg_any@10"] for r in rows) / n, ROUND) if n else 0.0,
        }

    return {
        "schema_version": SCHEMA_VERSION,
        "top_k": K,
        "per_type": {t: agg(rows) for t, rows in sorted(by_type.items())},
        "overall": agg(allrows),
    }


def validate(report: dict, reported_overall_accuracy: float | None) -> list[str]:
    errors = []
    present = set(report["per_type"])
    missing = set(LONGMEMEVAL_TYPES) - present
    if missing:
        errors.append(f"report missing dataset types: {sorted(missing)}")
    if reported_overall_accuracy is not None:
        recomputed = report["overall"]["qa_accuracy"]
        if abs(recomputed - reported_overall_accuracy) > OVERALL_MATCH_TOL:
            errors.append(
                f"recomputed overall accuracy {recomputed} != evaluator-reported "
                f"{reported_overall_accuracy}"
            )
    return errors


def load_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def self_test() -> int:
    errors = []
    reference, retrieval, evaluator = [], [], {}
    for ti, qtype in enumerate(LONGMEMEVAL_TYPES):
        for j in range(2):
            qid = f"q_{ti}_{j}"
            gold = [f"g_{ti}_{j}"]
            reference.append({"question_id": qid, "question_type": qtype, "gold_ids": gold})
            # Deterministic: first of each pair correct + gold retrieved, second wrong + missed.
            correct = j == 0
            retrieval.append(
                {"question_id": qid, "retrieved_ids": (gold if correct else ["x"]) + ["y", "z"]}
            )
            evaluator[qid] = correct

    report = package(reference, retrieval, evaluator)

    # Per-type: each type has 2 questions, one correct -> qa_accuracy 0.5.
    for qtype, stats in report["per_type"].items():
        if stats["qa_accuracy"] != 0.5:
            errors.append(f"{qtype} qa_accuracy {stats['qa_accuracy']} != 0.5")
        if stats["recall_all@10"] != 0.5:
            errors.append(f"{qtype} recall_all@10 {stats['recall_all@10']} != 0.5")

    # All types present; overall accuracy 0.5 matches the "reported" 0.5.
    errors.extend(validate(report, reported_overall_accuracy=0.5))
    # A wrong reported overall must be caught.
    if not validate(report, reported_overall_accuracy=0.9):
        errors.append("overall-mismatch assertion did not fire")
    # A missing type must be caught.
    short = package(reference[2:], retrieval[2:], evaluator)
    if not validate(short, None):
        errors.append("missing-type assertion did not fire")

    if errors:
        print("longmemeval-per-type-report self-test FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"longmemeval-per-type-report self-test passed: {report['overall']['n']} questions "
        f"packaged across all {len(LONGMEMEVAL_TYPES)} types; per-type aggregation + "
        "all-types-present + overall-matches assertions verified"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--reference", type=pathlib.Path)
    ap.add_argument("--retrieval-log", type=pathlib.Path)
    ap.add_argument("--evaluator-output", type=pathlib.Path,
                    help="JSON {question_id: is_correct} from the official evaluator")
    ap.add_argument("--reported-overall", type=float, help="evaluator's reported overall accuracy")
    ap.add_argument("--output", type=pathlib.Path)
    args = ap.parse_args()

    if args.self_test or not (args.reference and args.retrieval_log and args.evaluator_output):
        return self_test()

    report = package(
        load_jsonl(args.reference),
        load_jsonl(args.retrieval_log),
        json.loads(args.evaluator_output.read_text()),
    )
    errors = validate(report, args.reported_overall)
    if errors:
        print("longmemeval-per-type-report FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(f"packaged {report['overall']['n']} questions across {len(report['per_type'])} types")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
