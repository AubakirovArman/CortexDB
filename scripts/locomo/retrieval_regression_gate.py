#!/usr/bin/env python3
"""F3.4 (retrieval half): LoCoMo retrieval regression gate.

Scores a LoCoMo retrieval log (hit@1, hit@10) and fails if either regresses more
than 0.01 below the committed baseline (hit@1 0.3199, hit@10 0.6312). This is the
offline, deterministic half of F3.4; the QA end-to-end half
(`run_qa.py` / `check_qa_evidence.py`) needs a reader endpoint and is gated on
that key.

Two entry points (mirroring F3.1/F3.3):
  - FAST (offline, gate default): replay the committed mini retrieval log through
    the scorer and assert a deliberately-degraded log is caught, proving the
    hit@k math + regression detection without a metered run.
  - REAL: `--log` a LoCoMo retrieval replay (`locomo-cortexdb-retrieval`,
    --reset-db) + `--baseline` the committed baseline to gate a real run.

Dependency-free (stdlib only); deterministic; no network, no wall clock, no LLM.
"""

from __future__ import annotations

import argparse
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
FIX = REPO / "fixtures" / "benchmarks" / "locomo"
REFERENCE = FIX / "reference.jsonl"
MINI_LOG = FIX / "mini_retrieval_log.jsonl"
DEGRADED_LOG = FIX / "degraded_retrieval_log.jsonl"
BASELINE = FIX / "retrieval_baseline_v1.json"
MINI_BASELINE = FIX / "mini_baseline_v1.json"

# Committed LoCoMo baseline + per-metric tolerance (drop that fails the gate).
LOCOMO_BASELINE = {"hit@1": 0.3199, "hit@10": 0.6312}
TOLERANCES = {"hit@1": 0.01, "hit@10": 0.01}
ROUND = 6


def hit_at_k(ranked: list[str], gold: set[str], k: int) -> float:
    return 1.0 if any(doc in gold for doc in ranked[:k]) else 0.0


def load_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def score(reference: list[dict], log: list[dict]) -> dict:
    ranked_by_q = {row["question_id"]: row["retrieved_ids"] for row in log}
    hit1 = hit10 = 0.0
    n = len(reference)
    for ref in reference:
        ranked = ranked_by_q.get(ref["question_id"], [])
        gold = set(ref["gold_ids"])
        hit1 += hit_at_k(ranked, gold, 1)
        hit10 += hit_at_k(ranked, gold, 10)
    return {
        "schema_version": "cortexdb.locomo.retrieval.v1",
        "n": n,
        "metrics": {
            "hit@1": round(hit1 / n, ROUND) if n else 0.0,
            "hit@10": round(hit10 / n, ROUND) if n else 0.0,
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
    if hit_at_k(["a", "b"], {"c"}, 10) != 0.0 or hit_at_k(["a"], {"a"}, 1) != 1.0:
        errors.append("hit math")
    if hit_at_k(["x", "a"], {"a"}, 1) != 0.0:
        errors.append("hit@1 must miss when gold is at rank 2")

    committed = json.loads(BASELINE.read_text())
    for metric, expected in LOCOMO_BASELINE.items():
        if abs(committed.get(metric, -1) - expected) > 1e-9:
            errors.append(f"baseline {metric} {committed.get(metric)} != committed {expected}")

    reference = load_jsonl(REFERENCE)
    mini_baseline = json.loads(MINI_BASELINE.read_text())
    good = compare(score(reference, load_jsonl(MINI_LOG)), mini_baseline)
    if good:
        errors.append(f"good mini-log regressed: {good}")
    degraded = compare(score(reference, load_jsonl(DEGRADED_LOG)), mini_baseline)
    if not degraded:
        errors.append("degraded retrieval log was NOT caught as a regression")

    if errors:
        print("locomo-retrieval-regression self-test FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"locomo-retrieval-regression self-test passed: hit@k math verified; baseline locked "
        f"(hit@10 {LOCOMO_BASELINE['hit@10']:.4f}); degraded log caught "
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
        print("locomo-retrieval-regression-check FAILED")
        for r in regressions:
            print(f"  {r}")
        return 1
    print(f"locomo-retrieval-regression-check passed: {report['metrics']} >= baseline")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
