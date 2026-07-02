#!/usr/bin/env python3
"""A2.0 — embedding-model selection harness.

Deterministic retrieval-metric evaluator (recall@k, MRR@k, index size at i16)
plus a consistency check over the recorded candidate matrix
`fixtures/embedding/model_selection_v1.json`: it recomputes the winner from the
candidates by the fixture's own selection rule and asserts it equals the
recorded `chosen` profile, so the profile A2.1/A2.2 consume stays the actual
measured winner.

Usage:
  embedding_model_eval.py --self-test
  embedding_model_eval.py --report target/embedding-model-selection/report.json
  embedding_model_eval.py --live-eval corpus.jsonl queries.jsonl   # optional, offline math

`--self-test` needs no files. The default (or `--report`) validates the fixture.
"""

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FIXTURE = REPO / "fixtures/embedding/model_selection_v1.json"
DOC = REPO / "docs/EMBEDDING_MODEL_SELECTION.md"
VALID_METRICS = {"none", "dot_product", "cosine", "l2"}


def recall_at_k(ranked_ids, relevant, k):
    """Fraction of relevant ids present in the top-k ranked ids."""
    if not relevant:
        return 0.0
    top = ranked_ids[:k]
    hit = sum(1 for doc_id in relevant if doc_id in top)
    return hit / len(relevant)


def mrr_at_k(ranked_ids, relevant, k):
    """Reciprocal rank of the first relevant id within top-k (0 if none)."""
    for index, doc_id in enumerate(ranked_ids[:k], start=1):
        if doc_id in relevant:
            return 1.0 / index
    return 0.0


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def rank_by_dot(query_vector, corpus):
    """Ranks corpus [(doc_id, vector), ...] by descending dot product, with a
    deterministic doc_id tie-break so equal scores never reorder run to run."""
    scored = [(dot(query_vector, vector), doc_id) for doc_id, vector in corpus]
    scored.sort(key=lambda item: (-item[0], item[1]))
    return [doc_id for _, doc_id in scored]


def index_bytes_i16(n_vectors, dimension):
    """On-disk vector index size for `n_vectors` at `dimension` in i16 (2 B/lane)."""
    return n_vectors * dimension * 2


def winner_by_rule(candidates, tie_epsilon=0.5):
    """Applies the fixture selection rule: max overall_doc_recall, tie-broken by
    smaller index_bytes_per_vector."""
    best = None
    for candidate in candidates:
        if best is None:
            best = candidate
            continue
        delta = candidate["overall_doc_recall"] - best["overall_doc_recall"]
        if delta > tie_epsilon:
            best = candidate
        elif abs(delta) <= tie_epsilon and (
            candidate["index_bytes_per_vector"] < best["index_bytes_per_vector"]
        ):
            best = candidate
    return best


def validate_fixture(errors):
    if not FIXTURE.exists():
        errors.append(f"missing fixture: {FIXTURE}")
        return None
    data = json.loads(FIXTURE.read_text())
    if data.get("schema_version") != "cortexdb.embedding_model_selection.v1":
        errors.append("fixture schema_version must be cortexdb.embedding_model_selection.v1")
    candidates = data.get("candidates", [])
    if len(candidates) < 2:
        errors.append("selection needs at least 2 candidates")
    for candidate in candidates:
        if candidate.get("metric") not in VALID_METRICS:
            errors.append(f"candidate {candidate.get('name')}: invalid metric {candidate.get('metric')}")
        if candidate.get("dimension", -1) < 0:
            errors.append(f"candidate {candidate.get('name')}: dimension must be >= 0")
    chosen = data.get("chosen")
    if not chosen:
        errors.append("fixture must record a chosen profile")
        return data
    # The recorded choice must be the measured winner by the fixture's own rule.
    winner = winner_by_rule(candidates)
    if winner is None or winner.get("name") != chosen.get("name"):
        errors.append(
            f"chosen '{chosen.get('name')}' is not the winner by selection_rule "
            f"(winner: {winner.get('name') if winner else None})"
        )
    for field in ("model", "dimension", "metric"):
        if winner is not None and chosen.get(field) != winner.get(field):
            errors.append(f"chosen.{field} disagrees with winning candidate")
    if chosen.get("metric") not in VALID_METRICS or chosen.get("metric") == "none":
        errors.append("chosen.metric must be one of dot_product/cosine/l2")
    if not isinstance(chosen.get("dimension"), int) or chosen.get("dimension", 0) <= 0:
        errors.append("chosen.dimension must be a positive integer")
    if not chosen.get("model"):
        errors.append("chosen.model must be non-empty")
    # The doc must reference the chosen model so the write-up cannot drift.
    if DOC.exists() and chosen.get("model") and chosen["model"] not in DOC.read_text():
        errors.append(f"docs/EMBEDDING_MODEL_SELECTION.md must mention chosen model {chosen['model']}")
    return data


def self_test():
    # Two docs, orthogonal-ish; query aligns with doc "b".
    corpus = [("a", [10, 0]), ("b", [0, 10]), ("c", [7, 7])]
    ranked = rank_by_dot([0, 10], corpus)
    assert ranked == ["b", "c", "a"], ranked
    assert recall_at_k(ranked, {"b"}, 1) == 1.0
    assert recall_at_k(ranked, {"a"}, 1) == 0.0
    assert recall_at_k(ranked, {"a", "b"}, 2) == 0.5
    assert mrr_at_k(ranked, {"c"}, 3) == 0.5  # "c" is rank 2
    assert mrr_at_k(ranked, {"a"}, 1) == 0.0  # not in top-1
    assert index_bytes_i16(1000, 1024) == 2_048_000
    # Tie-break: equal recall -> smaller index wins.
    cands = [
        {"name": "big", "overall_doc_recall": 67.5, "index_bytes_per_vector": 4096},
        {"name": "small", "overall_doc_recall": 67.6, "index_bytes_per_vector": 1024},
    ]
    assert winner_by_rule(cands)["name"] == "small"  # within epsilon, smaller index
    cands2 = [
        {"name": "low", "overall_doc_recall": 55.7, "index_bytes_per_vector": 0},
        {"name": "high", "overall_doc_recall": 67.5, "index_bytes_per_vector": 2048},
    ]
    assert winner_by_rule(cands2)["name"] == "high"  # clear recall win
    print("OK: embedding_model_eval self-test passed")


def live_eval(corpus_path, queries_path):
    """Offline metric computation over provided JSONL (no network): corpus lines
    are {"doc_id","vector"}, query lines {"query_id","vector","relevant":[...]}. """
    corpus = []
    for line in Path(corpus_path).read_text().splitlines():
        if line.strip():
            row = json.loads(line)
            corpus.append((row["doc_id"], row["vector"]))
    recalls, mrrs = [], []
    for line in Path(queries_path).read_text().splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        ranked = rank_by_dot(row["vector"], corpus)
        relevant = set(row.get("relevant", []))
        recalls.append(recall_at_k(ranked, relevant, 10))
        mrrs.append(mrr_at_k(ranked, relevant, 10))
    n = max(len(recalls), 1)
    print(json.dumps({
        "queries": len(recalls),
        "recall_at_10": sum(recalls) / n,
        "mrr_at_10": sum(mrrs) / n,
    }, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--report", default=None)
    parser.add_argument("--live-eval", nargs=2, metavar=("CORPUS", "QUERIES"))
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return 0
    if args.live_eval:
        return live_eval(*args.live_eval)

    # Default / --report: run the self-test AND validate the recorded matrix.
    self_test()
    errors = []
    data = validate_fixture(errors)
    report = {
        "schema_version": "cortexdb.embedding_model_selection.report.v1",
        "ok": not errors,
        "chosen": (data or {}).get("chosen"),
        "candidate_count": len((data or {}).get("candidates", [])),
        "errors": errors,
    }
    if args.report:
        out = Path(args.report)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(report, indent=2) + "\n")
    if errors:
        print("ERROR: embedding model selection is inconsistent:")
        for error in errors:
            print(f"  {error}")
        return 1
    chosen = report["chosen"]
    print(
        f"OK: embedding model selected: {chosen['model']} "
        f"(dim={chosen['dimension']}, metric={chosen['metric']})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
