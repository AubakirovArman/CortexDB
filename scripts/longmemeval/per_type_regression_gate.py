#!/usr/bin/env python3
"""F3.1: LongMemEval per-type retrieval regression gate.

Scores a LongMemEval retrieval log per question type (recall_all@10 + ndcg_any@10)
against a committed per-type baseline and FAILS if any single type regresses by
more than 0.02 (2.0 points) or the overall by more than 0.01 (1.0 point).

Two entry points (per the master plan):
  - FAST (offline, this gate's default): replay the committed 25-row
    `mini_retrieval_log.jsonl` through the scorer, plus a deliberately-failing
    `degraded_retrieval_log.jsonl`, proving the metric math + regression detection
    without any metered run.
  - REAL: point `--log` at an official retrieval log and `--baseline` at the
    official per-type baseline (`per_type_retrieval_baseline_v1.json`, seeded from
    the official run) to gate a real run.

recall_all@10 = |gold ∩ top-10| / |gold| (fraction of a question's gold evidence
retrieved); ndcg_any@10 = nDCG@10 with binary relevance (any gold id counts).

Dependency-free (stdlib only); deterministic; no network, no wall clock, no LLM.
"""

from __future__ import annotations

import argparse
import json
import math
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
FIX = REPO / "fixtures" / "benchmarks" / "longmemeval"
REFERENCE = FIX / "reference.jsonl"
MINI_LOG = FIX / "mini_retrieval_log.jsonl"
DEGRADED_LOG = FIX / "degraded_retrieval_log.jsonl"
BASELINE = FIX / "per_type_retrieval_baseline_v1.json"

K = 10
PER_TYPE_TOL = 0.02  # 2.0 points
OVERALL_TOL = 0.01  # 1.0 point
ROUND = 6


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


def load_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def score(reference: list[dict], log: list[dict]) -> dict:
    """Per-type + overall recall_all@10 / ndcg_any@10 for a retrieval log."""
    ranked_by_q = {row["question_id"]: row["retrieved_ids"] for row in log}
    by_type: dict[str, list[dict]] = {}
    allrows: list[dict] = []
    for ref in reference:
        ranked = ranked_by_q.get(ref["question_id"], [])
        gold = set(ref["gold_ids"])
        row = {
            "recall_all@10": recall_all_at_k(ranked, gold, K),
            "ndcg_any@10": ndcg_any_at_k(ranked, gold, K),
        }
        by_type.setdefault(ref["question_type"], []).append(row)
        allrows.append(row)

    def agg(rows: list[dict]) -> dict:
        n = len(rows)
        return {
            "n": n,
            "recall_all@10": round(sum(r["recall_all@10"] for r in rows) / n, ROUND) if n else 0.0,
            "ndcg_any@10": round(sum(r["ndcg_any@10"] for r in rows) / n, ROUND) if n else 0.0,
        }

    return {
        "schema_version": "cortexdb.longmemeval.per_type_retrieval.v1",
        "top_k": K,
        "per_type": {t: agg(rows) for t, rows in sorted(by_type.items())},
        "overall": agg(allrows),
    }


def compare(report: dict, baseline: dict) -> list[str]:
    regressions = []
    for metric in ("recall_all@10", "ndcg_any@10"):
        base_o = baseline.get("overall", {}).get(metric)
        cur_o = report["overall"].get(metric)
        if base_o is not None and cur_o < base_o - OVERALL_TOL:
            regressions.append(f"overall.{metric}: {cur_o} < baseline {base_o} (tol -{OVERALL_TOL})")
    for qtype, cur in report["per_type"].items():
        base_t = baseline.get("per_type", {}).get(qtype)
        if base_t is None:
            regressions.append(f"per_type.{qtype}: no baseline entry (regenerate deliberately)")
            continue
        for metric in ("recall_all@10", "ndcg_any@10"):
            if cur[metric] < base_t.get(metric, 0.0) - PER_TYPE_TOL:
                regressions.append(
                    f"per_type.{qtype}.{metric}: {cur[metric]} < baseline "
                    f"{base_t.get(metric)} (tol -{PER_TYPE_TOL})"
                )
    return regressions


def self_test() -> int:
    reference = load_jsonl(REFERENCE)
    good = load_jsonl(MINI_LOG)
    degraded = load_jsonl(DEGRADED_LOG)
    baseline = json.loads(BASELINE.read_text())
    errors = []

    # The good mini-log must meet its own committed baseline (no regression).
    good_report = score(reference, good)
    good_regressions = compare(good_report, baseline)
    if good_regressions:
        errors.append(f"good mini-log unexpectedly regressed: {good_regressions}")

    # The degraded log must be caught as a regression end-to-end.
    degraded_regressions = compare(score(reference, degraded), baseline)
    if not degraded_regressions:
        errors.append("degraded retrieval log was NOT caught as a regression")

    # All dataset types present in the reference must appear in the baseline.
    ref_types = {r["question_type"] for r in reference}
    if ref_types - set(baseline.get("per_type", {})):
        errors.append(f"baseline missing types: {ref_types - set(baseline['per_type'])}")

    # Metric unit-checks.
    if recall_all_at_k(["a", "b"], {"a", "c"}, 10) != 0.5:
        errors.append("recall_all math")
    if abs(ndcg_any_at_k(["a", "b"], {"a", "b"}, 10) - 1.0) > 1e-12:
        errors.append("ndcg_any perfect-ranking math")

    if errors:
        print("longmemeval-per-type-regression self-test FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"longmemeval-per-type-regression self-test passed: {good_report['overall']['n']} "
        f"questions across {len(good_report['per_type'])} types meet baseline; degraded log "
        f"caught ({len(degraded_regressions)} regression(s), e.g. {degraded_regressions[0]})"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--reference", type=pathlib.Path, default=REFERENCE)
    ap.add_argument("--log", type=pathlib.Path)
    ap.add_argument("--baseline", type=pathlib.Path, default=BASELINE)
    ap.add_argument("--report", type=pathlib.Path)
    ap.add_argument("--generate-baseline", action="store_true")
    args = ap.parse_args()

    if args.generate_baseline:
        report = score(load_jsonl(args.reference), load_jsonl(args.log or MINI_LOG))
        args.baseline.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(f"wrote baseline {args.baseline.relative_to(REPO)}")
        return 0

    if args.self_test or args.log is None:
        return self_test()

    report = score(load_jsonl(args.reference), load_jsonl(args.log))
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    regressions = compare(report, json.loads(args.baseline.read_text()))
    if regressions:
        print("longmemeval-per-type-regression-check FAILED")
        for r in regressions:
            print(f"  {r}")
        return 1
    print(
        f"longmemeval-per-type-regression-check passed: {report['overall']['n']} questions, "
        f"{len(report['per_type'])} types >= baseline"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
