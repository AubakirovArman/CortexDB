#!/usr/bin/env python3
"""F3.3: MultiHop-RAG retrieval regression gate.

Scores a MultiHop-RAG retrieval log (Hits@10, Hits@4, MAP@10, MRR@10) and fails
if any metric regresses beyond tolerance below the committed balanced_50 baseline
(from the F1.2 v7 snapshot: Hits@10 1.0000, Hits@4 0.9545, MAP@10 0.4396,
MRR@10 0.7760).

Two entry points (mirroring F3.1):
  - FAST (offline, gate default): replay the committed mini retrieval log through
    the scorer and assert a deliberately-degraded log is caught — proving the IR
    metric math + regression detection without the external MultiHop scorer.
  - REAL: `--log` a balanced_50 retrieval replay + `--baseline` the committed
    balanced_50 baseline to gate a real run; parity with the official
    `retrieval_evaluate.py` is verified when the MultiHop repo is available.

Dependency-free (stdlib only); deterministic; no network, no wall clock, no LLM.
"""

from __future__ import annotations

import argparse
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
FIX = REPO / "fixtures" / "benchmarks" / "multihop_rag"
REFERENCE = FIX / "reference.jsonl"
MINI_LOG = FIX / "mini_retrieval_log.jsonl"
DEGRADED_LOG = FIX / "degraded_retrieval_log.jsonl"
BASELINE = FIX / "retrieval_baseline_v1.json"  # real balanced_50 baseline
MINI_BASELINE = FIX / "mini_baseline_v1.json"  # self-consistent, for the fast lane

# Committed balanced_50 baseline (F1.2 v7 snapshot) + per-metric tolerance.
BALANCED_50_BASELINE = {
    "hits@10": 1.0000,
    "hits@4": 0.9545,
    "map@10": 0.4396,
    "mrr@10": 0.7760,
}
TOLERANCES = {"hits@10": 0.0, "hits@4": 0.02, "map@10": 0.02, "mrr@10": 0.02}
ROUND = 6


def _topk(ranked: list[str], k: int) -> list[str]:
    return ranked[:k]


def hits_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    return 1.0 if any(doc in gold for doc in _topk(ranked, k)) else 0.0


def average_precision_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    if not gold:
        return 0.0
    hits, precision_sum = 0, 0.0
    for i, doc in enumerate(_topk(ranked, k), 1):
        if doc in gold:
            hits += 1
            precision_sum += hits / i
    return precision_sum / min(len(gold), k)


def reciprocal_rank_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    for i, doc in enumerate(_topk(ranked, k), 1):
        if doc in gold:
            return 1.0 / i
    return 0.0


def load_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def score(reference: list[dict], log: list[dict]) -> dict:
    ranked_by_q = {row["question_id"]: row["retrieved_ids"] for row in log}
    hits10 = hits4 = ap10 = rr10 = 0.0
    n = len(reference)
    for ref in reference:
        ranked = ranked_by_q.get(ref["question_id"], [])
        gold = set(ref["gold_ids"])
        hits10 += hits_at_k(ranked, gold, 10)
        hits4 += hits_at_k(ranked, gold, 4)
        ap10 += average_precision_at_k(ranked, gold, 10)
        rr10 += reciprocal_rank_at_k(ranked, gold, 10)
    return {
        "schema_version": "cortexdb.multihop_rag.retrieval.v1",
        "n": n,
        "metrics": {
            "hits@10": round(hits10 / n, ROUND) if n else 0.0,
            "hits@4": round(hits4 / n, ROUND) if n else 0.0,
            "map@10": round(ap10 / n, ROUND) if n else 0.0,
            "mrr@10": round(rr10 / n, ROUND) if n else 0.0,
        },
    }


def compare(report: dict, baseline: dict) -> list[str]:
    regressions = []
    for metric, tol in TOLERANCES.items():
        base = baseline.get(metric)
        cur = report["metrics"].get(metric)
        if base is not None and cur < base - tol:
            regressions.append(f"{metric}: {cur} < baseline {base} (tol -{tol})")
    return regressions


def self_test() -> int:
    errors = []
    # IR metric unit-checks.
    if hits_at_k(["a", "b"], {"c"}, 10) != 0.0 or hits_at_k(["a"], {"a"}, 10) != 1.0:
        errors.append("hits math")
    # gold at ranks 1 and 3 of a 2-gold question: AP = (1/1 + 2/3)/2.
    ap = average_precision_at_k(["a", "x", "b"], {"a", "b"}, 10)
    if abs(ap - (1.0 + 2.0 / 3.0) / 2.0) > 1e-9:
        errors.append(f"map math: {ap}")
    if reciprocal_rank_at_k(["x", "a"], {"a"}, 10) != 0.5:
        errors.append("mrr math")

    # The committed balanced_50 baseline values are locked (guard against drift).
    committed = json.loads(BASELINE.read_text())
    for metric, expected in BALANCED_50_BASELINE.items():
        if abs(committed.get(metric, -1) - expected) > 1e-9:
            errors.append(f"baseline {metric} {committed.get(metric)} != committed {expected}")

    # Fast lane: mini log meets its self-consistent baseline; degraded log caught.
    reference = load_jsonl(REFERENCE)
    mini_baseline = json.loads(MINI_BASELINE.read_text())
    good = compare(score(reference, load_jsonl(MINI_LOG)), mini_baseline)
    if good:
        errors.append(f"good mini-log regressed: {good}")
    degraded = compare(score(reference, load_jsonl(DEGRADED_LOG)), mini_baseline)
    if not degraded:
        errors.append("degraded retrieval log was NOT caught as a regression")

    if errors:
        print("multihop-retrieval-regression self-test FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"multihop-retrieval-regression self-test passed: IR metrics verified; balanced_50 "
        f"baseline locked (Hits@10 {BALANCED_50_BASELINE['hits@10']:.4f}); degraded log caught "
        f"({len(degraded)} regression(s), e.g. {degraded[0]})"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--reference", type=pathlib.Path, default=REFERENCE)
    ap.add_argument("--log", type=pathlib.Path)
    ap.add_argument("--baseline", type=pathlib.Path, default=BASELINE)
    ap.add_argument("--report", type=pathlib.Path)
    ap.add_argument("--generate-mini-baseline", action="store_true")
    args = ap.parse_args()

    if args.generate_mini_baseline:
        report = score(load_jsonl(args.reference), load_jsonl(MINI_LOG))
        MINI_BASELINE.write_text(json.dumps(report["metrics"], indent=2, sort_keys=True) + "\n")
        print(f"wrote {MINI_BASELINE.relative_to(REPO)}")
        return 0

    if args.self_test or args.log is None:
        return self_test()

    report = score(load_jsonl(args.reference), load_jsonl(args.log))
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    regressions = compare(report, json.loads(args.baseline.read_text()))
    if regressions:
        print("multihop-retrieval-regression-check FAILED")
        for r in regressions:
            print(f"  {r}")
        return 1
    print(f"multihop-retrieval-regression-check passed: {report['metrics']} >= baseline")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
