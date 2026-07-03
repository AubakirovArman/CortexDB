#!/usr/bin/env python3
"""F5.2: benchmark-score trend gate.

Compares the *latest* committed benchmark snapshot to the *previous* one per
benchmark and fails when a headline metric regresses beyond a per-benchmark
tolerance. It reuses the F2.2 judge-identity rule: a benchmark whose measuring
instrument includes an LLM judge (EnterpriseRAG-Bench) can only be trend-compared
when the SAME judge scored both snapshots — a cross-judge delta measures the
judges, not the system, so it is REFUSED (no numeric verdict), never silently
treated as a regression or an improvement.

Tolerances (allowed drop before the gate fails), from the master plan:
  - longmemeval_v1: accuracy -0.01, recall_all@10 -0.005
  - enterprise_rag_bench: combined -1.0 (SAME-JUDGE ONLY, via the F2.2 rule)
  - multihop_rag: hits@10 -0.02
  - locomo: hit@10 -0.01

A judge/reader change on a judge-guarded benchmark resets the comparison
(comparable=false, judge_changed=true) — a deliberate re-baseline, not a
regression — so it does not fail the gate but is surfaced for a human.

Dependency-free (stdlib only); deterministic; no network, no wall clock.
"""

from __future__ import annotations

import argparse
import json
import pathlib

# benchmark_id -> {"judge_guarded": bool, "tolerances": {metric: allowed_drop}}
TREND_RULES = {
    "longmemeval_v1": {
        "judge_guarded": False,
        "tolerances": {"accuracy": 0.01, "recall_all@10": 0.005},
    },
    "enterprise_rag_bench": {
        "judge_guarded": True,
        "tolerances": {"combined_correctness_completeness_score": 1.0},
    },
    "multihop_rag": {
        "judge_guarded": False,
        "tolerances": {"hits@10": 0.02},
    },
    "locomo": {
        "judge_guarded": False,
        "tolerances": {"hit@10": 0.01},
    },
}


def judge_identity(snapshot: dict) -> tuple:
    """Judge (model, provider), tolerating both the registry shape
    (`judge`: {model, provider}) and the flat run shape."""
    judge = snapshot.get("judge")
    if isinstance(judge, dict):
        return (judge.get("model"), judge.get("provider"))
    return (snapshot.get("judge_model"), snapshot.get("judge_provider"))


def metric_value(snapshot: dict, metric: str):
    """Read a metric from `metrics` (registry shape) or the top level."""
    metrics = snapshot.get("metrics", {})
    if metric in metrics:
        return metrics[metric]
    return snapshot.get(metric)


def compare_trend(benchmark_id: str, previous: dict, latest: dict) -> dict:
    rule = TREND_RULES.get(benchmark_id)
    if rule is None:
        return {
            "schema_version": "cortexdb.benchmark_trend.v1",
            "benchmark_id": benchmark_id,
            "comparable": False,
            "reason": f"no trend rule for benchmark {benchmark_id}",
            "regressions": [],
        }

    if rule["judge_guarded"]:
        jp, jl = judge_identity(previous), judge_identity(latest)
        if jp != jl:
            # Cross-judge: refuse the numeric trend (F2.2 rule). No deltas leaked.
            return {
                "schema_version": "cortexdb.benchmark_trend.v1",
                "benchmark_id": benchmark_id,
                "comparable": False,
                "judge_changed": True,
                "reason": (
                    f"judge changed: previous={jp[0]}/{jp[1]} vs latest={jl[0]}/{jl[1]}; "
                    "a cross-judge trend measures the judges, not the system — re-baseline"
                ),
                "regressions": [],
            }

    deltas = {}
    regressions = []
    for metric, tolerance in sorted(rule["tolerances"].items()):
        prev_v, latest_v = metric_value(previous, metric), metric_value(latest, metric)
        if prev_v is None or latest_v is None:
            continue
        delta = round(float(latest_v) - float(prev_v), 6)
        deltas[metric] = delta
        if delta < -tolerance:
            regressions.append(
                {
                    "metric": metric,
                    "previous": float(prev_v),
                    "latest": float(latest_v),
                    "delta": delta,
                    "tolerance": -tolerance,
                }
            )
    return {
        "schema_version": "cortexdb.benchmark_trend.v1",
        "benchmark_id": benchmark_id,
        "comparable": True,
        "judge_changed": False,
        "deltas": deltas,
        "regressions": regressions,
    }


def self_test() -> int:
    # LongMemEval: within tolerance passes; beyond fails.
    prev = {"metrics": {"accuracy": 0.766, "recall_all@10": 0.9021}}
    ok = {"metrics": {"accuracy": 0.760, "recall_all@10": 0.9000}}  # -0.006 / -0.0021, within tol
    bad = {"metrics": {"accuracy": 0.750, "recall_all@10": 0.8900}}  # -0.016 / -0.0121, beyond tol
    r_ok = compare_trend("longmemeval_v1", prev, ok)
    r_bad = compare_trend("longmemeval_v1", prev, bad)
    assert r_ok["comparable"] and not r_ok["regressions"], r_ok
    assert r_bad["regressions"], "beyond-tolerance drop must regress"
    assert {reg["metric"] for reg in r_bad["regressions"]} == {"accuracy", "recall_all@10"}, r_bad

    # Improvement never regresses.
    up = {"metrics": {"accuracy": 0.80, "recall_all@10": 0.95}}
    assert not compare_trend("longmemeval_v1", prev, up)["regressions"]

    # ERB judge-guarded: same judge compares; a >1.0 combined drop regresses.
    ejp = {"judge": {"model": "gpt-5.4", "provider": "openai"},
           "metrics": {"combined_correctness_completeness_score": 60.0}}
    esame_bad = {"judge": {"model": "gpt-5.4", "provider": "openai"},
                 "metrics": {"combined_correctness_completeness_score": 58.5}}  # -1.5, beyond -1.0
    r_erb = compare_trend("enterprise_rag_bench", ejp, esame_bad)
    assert r_erb["comparable"] and r_erb["regressions"], "same-judge ERB drop must regress"

    # ERB cross-judge: refuse, NO regression verdict, no deltas leaked.
    ediff = {"judge": {"model": "gemini-3.5-flash", "provider": "google"},
             "metrics": {"combined_correctness_completeness_score": 40.0}}
    r_cross = compare_trend("enterprise_rag_bench", ejp, ediff)
    assert r_cross["comparable"] is False and r_cross["judge_changed"] is True, r_cross
    assert not r_cross["regressions"], "a judge change must not be reported as a regression"
    assert "deltas" not in r_cross, "refused trend must not leak deltas"

    # MultiHop + LoCoMo tolerances.
    assert compare_trend(
        "multihop_rag", {"metrics": {"hits@10": 0.50}}, {"metrics": {"hits@10": 0.47}}
    )["regressions"], "multihop -0.03 must regress (tol -0.02)"
    assert not compare_trend(
        "locomo", {"metrics": {"hit@10": 0.60}}, {"metrics": {"hit@10": 0.595}}
    )["regressions"], "locomo -0.005 within tol (-0.01) must pass"

    # Determinism.
    assert compare_trend("longmemeval_v1", prev, bad) == r_bad
    print(
        "benchmark-trend self-test passed: per-benchmark tolerances enforced; "
        "ERB cross-judge trend refused (no delta / no false regression); improvements pass"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--benchmark")
    ap.add_argument("--previous", type=pathlib.Path)
    ap.add_argument("--latest", type=pathlib.Path)
    ap.add_argument("--report", type=pathlib.Path)
    ap.add_argument("--strict", action="store_true", help="exit non-zero on a regression")
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    if not (args.benchmark and args.previous and args.latest):
        ap.error("--benchmark, --previous, --latest are required unless --self-test")
    result = compare_trend(
        args.benchmark,
        json.loads(args.previous.read_text()),
        json.loads(args.latest.read_text()),
    )
    text = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text)
    print(text, end="")
    if args.strict and result.get("regressions"):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
