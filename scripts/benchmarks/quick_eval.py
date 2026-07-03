#!/usr/bin/env python3
"""F1.4 — fast shared retrieval-eval loop for ranking tasks.

A single deterministic, offline harness (`make quick-retrieval-eval`) that runs
**real** CortexDB keyword retrieval over a cached self-contained mini-corpus and
scores per-question-type retrieval quality (recall@k, MRR, nDCG@k). It compares
the result to a committed registry baseline and fails on regression, so every
Track A/B ranking change can run one uniform check instead of a bespoke A/B
procedure.

Properties (the F1.4 acceptance):
- **Real ranking, not a cached log.** Retrieval is executed through the
  `cortexdb` CLI (ingest each corpus doc as a scoped cell, run one keyword search
  per question, read the ranked results), so a ranking change moves the numbers.
- **Deterministic.** Keyword BM25 has no wall-clock/RNG; the report carries no
  timestamps, so a double run is byte-identical.
- **Offline & fast.** No LLM, no network; a ~12-doc corpus runs in well under the
  5-minute budget.
- **Degradation is caught.** `--self-test` checks the metric math on known
  rankings and proves a degraded ranking scores below the baseline (the gate would
  fail), so the harness demonstrably detects regressions.

Modes:
  (default)            run retrieval, write the report, compare to --baseline, exit 1 on regression
  --generate-baseline  run retrieval and (re)write --baseline from the current metrics
  --self-test          unit-check the metric functions + degradation detection (no CLI needed)
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCHEMA_VERSION = "cortexdb.benchmarks.quick_retrieval_eval.v1"
SCOPE = "quickeval"
# A regression must clear rounding noise; metrics are rounded to ROUND places and
# a drop beyond EPS below the committed baseline fails the gate.
ROUND = 6
EPS = 1e-6


# ---- metrics (pure; unit-tested in --self-test) ----------------------------


def recall_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    if not gold:
        return 0.0
    topk = ranked[:k]
    hit = sum(1 for doc in gold if doc in topk)
    return hit / len(gold)


def reciprocal_rank(ranked: list[str], gold: set[str], k: int) -> float:
    for index, doc in enumerate(ranked[:k], start=1):
        if doc in gold:
            return 1.0 / index
    return 0.0


def ndcg_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    if not gold:
        return 0.0
    dcg = 0.0
    for index, doc in enumerate(ranked[:k], start=1):
        if doc in gold:
            dcg += 1.0 / math.log2(index + 1)
    ideal_hits = min(len(gold), k)
    idcg = sum(1.0 / math.log2(index + 1) for index in range(1, ideal_hits + 1))
    return dcg / idcg if idcg else 0.0


def aggregate(rows: list[dict], k: int) -> dict:
    """Mean recall@k / MRR / nDCG@k over a set of scored questions."""
    n = len(rows)
    if n == 0:
        return {"n": 0, "recall_at_k": 0.0, "mrr": 0.0, "ndcg_at_k": 0.0}
    return {
        "n": n,
        "recall_at_k": round(sum(r["recall"] for r in rows) / n, ROUND),
        "mrr": round(sum(r["rr"] for r in rows) / n, ROUND),
        "ndcg_at_k": round(sum(r["ndcg"] for r in rows) / n, ROUND),
    }


def score_questions(question_rankings: list[tuple[str, list[str], set[str]]], k: int) -> dict:
    """question_rankings: (question_type, ranked_doc_ids, gold_doc_ids). Returns a
    deterministic report: per-type + overall recall@k / MRR / nDCG@k."""
    by_type: dict[str, list[dict]] = {}
    all_rows: list[dict] = []
    for qtype, ranked, gold in question_rankings:
        row = {
            "recall": recall_at_k(ranked, gold, k),
            "rr": reciprocal_rank(ranked, gold, k),
            "ndcg": ndcg_at_k(ranked, gold, k),
        }
        by_type.setdefault(qtype, []).append(row)
        all_rows.append(row)
    return {
        "schema_version": SCHEMA_VERSION,
        "top_k": k,
        "per_type": {qtype: aggregate(rows, k) for qtype, rows in sorted(by_type.items())},
        "overall": aggregate(all_rows, k),
    }


# ---- retrieval via the cortexdb CLI ----------------------------------------


def load_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def citation_of(payload: str) -> str | None:
    for line in payload.splitlines():
        if line.startswith("citation="):
            return line[len("citation=") :].strip()
    return None


def run_retrieval(binary: Path, corpus: list[dict], questions: list[dict], k: int) -> list[tuple]:
    """Ingest the corpus into a throwaway db and run one keyword search per
    question, returning (question_type, ranked_doc_ids, gold) tuples."""
    with tempfile.TemporaryDirectory() as tmp:
        db = str(Path(tmp) / "quickeval-db")

        def cli(*args: str) -> str:
            result = subprocess.run(
                [str(binary), *args],
                capture_output=True,
                text=True,
                check=True,
            )
            return result.stdout

        cli("init", db)
        for index, doc in enumerate(corpus):
            payload = f"scope={SCOPE}\ncitation={doc['doc_id']}\n\n{doc['body']}"
            cli("put", db, str(1000 + index), payload)

        tuples = []
        for question in questions:
            out = cli("search", db, SCOPE, question["question"], "--mode", "keyword", "--json")
            results = json.loads(out).get("results", [])
            ranked = []
            for entry in results:
                doc_id = citation_of(entry.get("payload", ""))
                if doc_id is not None:
                    ranked.append(doc_id)
            tuples.append(
                (question["question_type"], ranked, set(question["expected_doc_ids"]))
            )
        return tuples


# ---- baseline comparison ---------------------------------------------------


def compare_to_baseline(report: dict, baseline: dict) -> list[str]:
    """A regression is any per-type or overall metric that drops more than EPS
    below the committed baseline. New question types with no baseline entry are a
    regression (the baseline must be regenerated deliberately)."""
    regressions = []
    metrics = ("recall_at_k", "mrr", "ndcg_at_k")

    def check(scope: str, current: dict, base: dict | None) -> None:
        if base is None:
            regressions.append(f"{scope}: no baseline entry (regenerate baseline deliberately)")
            return
        for metric in metrics:
            if current[metric] < base.get(metric, 0.0) - EPS:
                regressions.append(
                    f"{scope}.{metric}: {current[metric]} < baseline {base.get(metric)}"
                )

    check("overall", report["overall"], baseline.get("overall"))
    for qtype, current in report["per_type"].items():
        check(f"per_type.{qtype}", current, baseline.get("per_type", {}).get(qtype))
    return regressions


# ---- self-test (metric math + degradation detection) -----------------------


def self_test() -> int:
    errors = []

    # recall@k
    if recall_at_k(["a", "b", "c"], {"a", "c"}, 10) != 1.0:
        errors.append("recall: full recall expected")
    if recall_at_k(["x", "a"], {"a", "b"}, 10) != 0.5:
        errors.append("recall: half recall expected")
    if recall_at_k(["a"], {"a"}, 0) != 0.0:
        errors.append("recall@0 must be 0")

    # reciprocal rank
    if reciprocal_rank(["x", "y", "a"], {"a"}, 10) != 1.0 / 3.0:
        errors.append("rr: 1/3 expected")
    if reciprocal_rank(["x", "y"], {"a"}, 10) != 0.0:
        errors.append("rr: miss must be 0")

    # nDCG@k: gold at ranks 1 and 3 vs ideal at ranks 1,2
    got = ndcg_at_k(["a", "x", "b"], {"a", "b"}, 10)
    dcg = 1.0 / math.log2(2) + 1.0 / math.log2(4)
    idcg = 1.0 / math.log2(2) + 1.0 / math.log2(3)
    if abs(got - dcg / idcg) > 1e-12:
        errors.append(f"ndcg: {got} != {dcg / idcg}")
    if abs(ndcg_at_k(["a", "b"], {"a", "b"}, 10) - 1.0) > 1e-12:
        errors.append("ndcg: perfect ranking must be 1.0")

    # Degradation detection: a good ranking meets the baseline; a degraded ranking
    # (gold pushed out of top-k) scores below it and would fail the gate.
    k = 3
    good = [("basic", ["g1", "x", "y"], {"g1"}), ("basic", ["g2", "x", "y"], {"g2"})]
    degraded = [("basic", ["x", "y", "z"], {"g1"}), ("basic", ["x", "y", "z"], {"g2"})]
    good_report = score_questions(good, k)
    degraded_report = score_questions(degraded, k)
    baseline = {"overall": good_report["overall"], "per_type": good_report["per_type"]}
    if compare_to_baseline(good_report, baseline):
        errors.append("degradation: good ranking must not regress against its own baseline")
    caught = compare_to_baseline(degraded_report, baseline)
    if not caught:
        errors.append("degradation: degraded ranking must be caught as a regression")

    if errors:
        print("quick-retrieval-eval self-test FAILED")
        for error in errors:
            print(f"  {error}")
        return 1
    print("quick-retrieval-eval self-test passed: metric math + degradation detection verified")
    return 0


# ---- main ------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(description="F1.4 quick retrieval eval")
    parser.add_argument("--bin", default="target/debug/cortexdb", help="cortexdb binary path")
    parser.add_argument("--corpus", default="fixtures/benchmarks/quick_eval/corpus.jsonl")
    parser.add_argument("--questions", default="fixtures/benchmarks/quick_eval/questions.jsonl")
    parser.add_argument(
        "--baseline", default="fixtures/benchmarks/quick_eval/registry_baseline_v1.json"
    )
    parser.add_argument("--report", default="target/quick-retrieval-eval/report.json")
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--generate-baseline", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--degradation-check",
        action="store_true",
        help="run retrieval over the committed degraded corpus and assert the gate catches it",
    )
    parser.add_argument(
        "--degraded-corpus", default="fixtures/benchmarks/quick_eval/degraded_corpus.jsonl"
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    binary = (REPO / args.bin) if not Path(args.bin).is_absolute() else Path(args.bin)
    if not binary.exists():
        print(f"missing cortexdb binary {binary} (run: cargo build -p cortex-cli --bin cortexdb)")
        return 1

    def resolve(rel: str) -> Path:
        return REPO / rel if not Path(rel).is_absolute() else Path(rel)

    questions = load_jsonl(resolve(args.questions))
    baseline_path = resolve(args.baseline)

    if args.degradation_check:
        # End-to-end proof that the gate catches a real retrieval degradation: run
        # the SAME questions over a committed degraded corpus (gold docs stripped
        # of their distinctive terms) and require the result to regress below the
        # committed baseline. If it does NOT regress, the gate is not protective.
        if not baseline_path.exists():
            print(f"missing baseline {baseline_path} (run with --generate-baseline)")
            return 1
        baseline = json.loads(baseline_path.read_text())
        degraded = load_jsonl(resolve(args.degraded_corpus))
        degraded_report = score_questions(
            run_retrieval(binary, degraded, questions, args.top_k), args.top_k
        )
        regressions = compare_to_baseline(degraded_report, baseline)
        if not regressions:
            print(
                "quick-retrieval-eval degradation-check FAILED: the degraded corpus did "
                "NOT regress below baseline — the gate would miss a real regression"
            )
            return 1
        print(
            f"quick-retrieval-eval degradation-check passed: degraded corpus caught "
            f"({len(regressions)} regression(s), e.g. {regressions[0]})"
        )
        return 0

    corpus = load_jsonl(resolve(args.corpus))
    tuples = run_retrieval(binary, corpus, questions, args.top_k)
    report = score_questions(tuples, args.top_k)

    report_path = REPO / args.report if not Path(args.report).is_absolute() else Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")

    baseline_path = (
        REPO / args.baseline if not Path(args.baseline).is_absolute() else Path(args.baseline)
    )
    if args.generate_baseline:
        baseline_path.parent.mkdir(parents=True, exist_ok=True)
        baseline_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(f"wrote baseline {baseline_path.relative_to(REPO)}")
        return 0

    if not baseline_path.exists():
        print(f"missing baseline {baseline_path} (run with --generate-baseline)")
        return 1
    baseline = json.loads(baseline_path.read_text())
    regressions = compare_to_baseline(report, baseline)
    if regressions:
        print("quick-retrieval-eval-check FAILED: retrieval regressed below the registry baseline")
        for regression in regressions:
            print(f"  {regression}")
        return 1
    overall = report["overall"]
    print(
        f"quick-retrieval-eval-check passed: {overall['n']} questions, "
        f"{len(report['per_type'])} types; overall recall@{args.top_k}="
        f"{overall['recall_at_k']} mrr={overall['mrr']} ndcg={overall['ndcg_at_k']} "
        "(>= committed baseline)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
