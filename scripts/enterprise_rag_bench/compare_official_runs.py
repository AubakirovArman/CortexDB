#!/usr/bin/env python3
"""F2.2: same-judge guard for EnterpriseRAG-Bench run comparisons.

Two ERB runs can only be compared if the SAME judge scored both — an LLM judge is
part of the measuring instrument, so a delta between a gemini-judged run and a
gpt-judged run measures the judges, not the systems. This comparator enforces
that: given two official_results.json-shaped reports, if their
(judge_model, judge_provider) differ it REFUSES — emitting
`refused_cross_judge_comparison: true` with NO delta fields (and, under
`--strict`, a non-zero exit). Only when the judge matches does it emit the
headline + per-question-type deltas.

This is the dynamic complement to the F1.2 registry's static `judge.official`
guard: the registry stops an unofficial number from being *published* as
comparable; this stops two runs from being *compared* across judges at all.

Dependency-free (stdlib only); deterministic; no network, no wall clock.
"""

from __future__ import annotations

import argparse
import json
import pathlib

HEADLINE = "combined_correctness_completeness_score"
PER_TYPE_METRICS = [
    "combined_correctness_completeness_score",
    "average_recall_pct",
    "average_correctness_pct",
    "average_completeness_pct",
]


def judge_identity(report: dict) -> tuple:
    return (report.get("judge_model"), report.get("judge_provider"))


def compare(baseline: dict, candidate: dict) -> dict:
    jb, jc = judge_identity(baseline), judge_identity(candidate)
    if jb != jc:
        # Cross-judge: refuse with NO deltas — a delta here would be a lie.
        return {
            "schema_version": "cortexdb.erb_run_comparison.v1",
            "comparable": False,
            "refused_cross_judge_comparison": True,
            "reason": (
                f"judge mismatch: baseline={jb[0]}/{jb[1]} vs candidate={jc[0]}/{jc[1]}; "
                "a cross-judge delta measures the judges, not the systems"
            ),
            "baseline_judge": {"model": jb[0], "provider": jb[1]},
            "candidate_judge": {"model": jc[0], "provider": jc[1]},
        }
    ab = baseline.get("aggregate_stats", {})
    ac = candidate.get("aggregate_stats", {})
    headline_delta = None
    if HEADLINE in ab and HEADLINE in ac:
        headline_delta = round(float(ac[HEADLINE]) - float(ab[HEADLINE]), 4)
    per_type = {}
    tb = baseline.get("question_type_stats", {})
    tc = candidate.get("question_type_stats", {})
    for qtype in sorted(set(tb) & set(tc)):
        row = {}
        for m in PER_TYPE_METRICS:
            if m in tb[qtype] and m in tc[qtype]:
                row[m] = round(float(tc[qtype][m]) - float(tb[qtype][m]), 4)
        per_type[qtype] = row
    return {
        "schema_version": "cortexdb.erb_run_comparison.v1",
        "comparable": True,
        "refused_cross_judge_comparison": False,
        "judge": {"model": jc[0], "provider": jc[1]},
        "headline_metric": HEADLINE,
        "headline_delta": headline_delta,
        "per_type_delta": per_type,
    }


def self_test() -> int:
    base = {
        "judge_model": "gemini-3.5-flash", "judge_provider": "google",
        "aggregate_stats": {HEADLINE: 47.74},
        "question_type_stats": {"basic": {m: 50.0 for m in PER_TYPE_METRICS}},
    }
    same = {
        "judge_model": "gemini-3.5-flash", "judge_provider": "google",
        "aggregate_stats": {HEADLINE: 49.24},
        "question_type_stats": {"basic": {m: 52.0 for m in PER_TYPE_METRICS}},
    }
    diff = {
        "judge_model": "gpt-5.4", "judge_provider": "openai",
        "aggregate_stats": {HEADLINE: 60.0},
        "question_type_stats": {"basic": {m: 61.0 for m in PER_TYPE_METRICS}},
    }
    r_same = compare(base, same)
    assert r_same["comparable"] is True, "same judge must be comparable"
    assert r_same["headline_delta"] == 1.5, r_same["headline_delta"]
    assert r_same["per_type_delta"]["basic"][HEADLINE] == 2.0
    assert "refused" not in json.dumps(r_same) or r_same["refused_cross_judge_comparison"] is False

    r_diff = compare(base, diff)
    assert r_diff["comparable"] is False, "cross-judge must not be comparable"
    assert r_diff["refused_cross_judge_comparison"] is True
    # The critical invariant: a refused comparison carries NO delta fields.
    assert "headline_delta" not in r_diff, "refused comparison must not leak a delta"
    assert "per_type_delta" not in r_diff, "refused comparison must not leak per-type deltas"

    # Determinism.
    assert compare(base, same) == r_same and compare(base, diff) == r_diff
    print(
        "erb-compare-runs self-test passed: same-judge delta "
        f"{r_same['headline_delta']:+.2f}; cross-judge correctly refused with no delta leaked"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--baseline", type=pathlib.Path)
    ap.add_argument("--candidate", type=pathlib.Path)
    ap.add_argument("--report", type=pathlib.Path)
    ap.add_argument("--strict", action="store_true",
                    help="exit non-zero when the comparison is refused (cross-judge)")
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    if not (args.baseline and args.candidate):
        ap.error("--baseline and --candidate are required unless --self-test")
    result = compare(
        json.loads(args.baseline.read_text()),
        json.loads(args.candidate.read_text()),
    )
    text = json.dumps(result, indent=2) + "\n"
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text)
    print(text, end="")
    if args.strict and result["refused_cross_judge_comparison"]:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
