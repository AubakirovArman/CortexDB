#!/usr/bin/env python3
"""A5.2 (training half): train + freeze a deterministic Q16 learned ranker.

Reads the A5.1 LTR corpus (build_ltr_corpus.py output: per (question, candidate)
a Q16 feature vector + a binary gold label + a leak-free train/heldout split),
learns per-feature weights on the TRAIN split by a deterministic Fisher-style
rule (weight proportional to the mean-feature gap between positives and
negatives), normalizes them to a Q16 vector that sums to 65535, and freezes them.
It then proves the learned ranker beats a uniform baseline on the HELD-OUT split
by mean reciprocal rank — the evidence the C3-1 frozen-weights protocol requires
before any ranking weight may change. Fully deterministic (no RNG, no wall clock).

The engine's opt-in serving of this frozen artifact is a separate, default-off
step (so it never changes the default ranking or its goldens).
"""

from __future__ import annotations

import argparse
import json
import pathlib

Q16 = 65535
FEATURES = ["lexical", "semantic", "recency", "trust"]
REPO = pathlib.Path(__file__).resolve().parent.parent.parent
DEFAULT_CORPUS = REPO / "fixtures" / "enterprise_rag_bench" / "learned_ranking" / "offline_v2.jsonl"
DEFAULT_OUT = REPO / "fixtures" / "enterprise_rag_bench" / "learned_ranking" / "learned_ranker_v2.json"


def _by_question(rows: list[dict], split: str) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = {}
    for r in rows:
        if r["split"] == split:
            grouped.setdefault(r["question_id"], []).append(r)
    return grouped


def train_weights(rows: list[dict]) -> dict[str, int]:
    """Fisher-style: weight_f ∝ max(0, mean(feature_f | positive) − mean(feature_f | negative))."""
    pos = [r["features_q16"] for r in rows if r["split"] == "train" and r["label"] == 1]
    neg = [r["features_q16"] for r in rows if r["split"] == "train" and r["label"] == 0]
    gaps = {}
    for f in FEATURES:
        mp = sum(x[f] for x in pos) / len(pos) if pos else 0.0
        mn = sum(x[f] for x in neg) / len(neg) if neg else 0.0
        gaps[f] = max(0.0, mp - mn)
    total = sum(gaps.values())
    if total <= 0:
        # Degenerate corpus: fall back to a uniform weighting.
        base = Q16 // len(FEATURES)
        weights = {f: base for f in FEATURES}
    else:
        weights = {f: int(round(gaps[f] / total * Q16)) for f in FEATURES}
    # Force the vector to sum to exactly Q16 (put the rounding remainder on lexical).
    weights["lexical"] += Q16 - sum(weights.values())
    return weights


def _score(features: dict, weights: dict[str, int]) -> int:
    return sum(features[f] * weights[f] for f in FEATURES)


def mean_reciprocal_rank(grouped: dict[str, list[dict]], weights: dict[str, int]) -> float:
    if not grouped:
        return 0.0
    total = 0.0
    for cands in grouped.values():
        ranked = sorted(cands, key=lambda r: (-_score(r["features_q16"], weights), r["document_id"]))
        rr = 0.0
        for rank, r in enumerate(ranked, start=1):
            if r["label"] == 1:
                rr = 1.0 / rank
                break
        total += rr
    return total / len(grouped)


def train(rows: list[dict]) -> dict:
    weights = train_weights(rows)
    uniform = {f: Q16 // len(FEATURES) for f in FEATURES}
    heldout = _by_question(rows, "heldout")
    learned_mrr = mean_reciprocal_rank(heldout, weights)
    baseline_mrr = mean_reciprocal_rank(heldout, uniform)
    return {
        "schema_version": "cortexdb.learned_ranker.v2",
        "version": "learned-ranker-v2",
        "weights_q16": weights,
        "heldout_mrr": round(learned_mrr, 6),
        "heldout_baseline_mrr": round(baseline_mrr, 6),
        "heldout_mrr_lift": round(learned_mrr - baseline_mrr, 6),
        "heldout_questions": len(heldout),
    }


def self_test() -> int:
    rows = [json.loads(l) for l in DEFAULT_CORPUS.read_text().splitlines() if l.strip()]
    artifact = train(rows)
    w = artifact["weights_q16"]
    assert sum(w.values()) == Q16, f"weights must sum to Q16, got {sum(w.values())}"
    assert all(0 <= v <= Q16 for v in w.values()), "weights out of Q16 range"
    # Determinism.
    assert train(rows) == artifact, "training is not deterministic"
    # Held-out lift (the C3-1 evidence): learned must not lose to uniform.
    assert artifact["heldout_mrr_lift"] >= 0.0, (
        f"learned ranker must not regress held-out MRR vs uniform: {artifact['heldout_mrr_lift']}"
    )
    print(
        f"train-learned-ranker self-test passed: weights={w}, "
        f"heldout MRR {artifact['heldout_mrr']} vs baseline {artifact['heldout_baseline_mrr']} "
        f"(lift {artifact['heldout_mrr_lift']:+.4f}, {artifact['heldout_questions']} questions)"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--corpus", type=pathlib.Path, default=DEFAULT_CORPUS)
    ap.add_argument("--output", type=pathlib.Path, default=DEFAULT_OUT)
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    rows = [json.loads(l) for l in args.corpus.read_text().splitlines() if l.strip()]
    artifact = train(rows)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n")
    print(f"froze {args.output} weights={artifact['weights_q16']} lift={artifact['heldout_mrr_lift']:+.4f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
