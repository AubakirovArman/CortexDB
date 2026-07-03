#!/usr/bin/env python3
"""A5.1: build a deterministic offline learned-to-rank (LTR) corpus.

For each (question, candidate) pair it emits the A1.1-normalized Q16 component
vector (lexical / semantic / recency / trust, min-max normalized per question
exactly as `min_max_normalize_q16` does) plus a binary gold label. It then splits
the corpus train/heldout with **no leakage on either axis**: two questions that
share any document are placed in the same split (document-connected components),
so no `question_id` and no `document_id` crosses the split. The build is
byte-deterministic (stable ordering, no wall clock, no RNG), so the corpus and
its manifest hash are reproducible.

Modes:
  --self-test                 build a synthetic corpus and assert the invariants
  --input <jsonl> --output    build from real retrieval traces
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys

Q16 = 65535


def normalize_q16(values: list[int]) -> list[int]:
    """min-max to [0, Q16]; no spread -> all Q16 (matches min_max_normalize_q16)."""
    if not values:
        return []
    lo, hi = min(values), max(values)
    if hi <= lo:
        return [Q16 for _ in values]
    span = hi - lo
    return [((v - lo) * Q16) // span for v in values]


def _component_split(rows: list[dict], heldout_ratio: float) -> dict[str, str]:
    """Union-find over questions sharing a document_id; assign whole components to
    a split by a stable hash so no document/question crosses train<->heldout."""
    parent: dict[str, str] = {}

    def find(x: str) -> str:
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a: str, b: str) -> None:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[min(ra, rb)] = max(ra, rb)  # deterministic: smaller id is root's child

    doc_to_q: dict[str, str] = {}
    for row in rows:
        qid = row["question_id"]
        find(qid)
        for cand in row["candidates"]:
            doc = cand["document_id"]
            if doc in doc_to_q:
                union(qid, doc_to_q[doc])
            else:
                doc_to_q[doc] = qid

    # Assign each component (by its root) to a split via a stable hash of the root.
    split_of: dict[str, str] = {}
    for row in rows:
        root = find(row["question_id"])
        if root not in split_of:
            h = int(hashlib.sha256(root.encode()).hexdigest(), 16) % 1000
            split_of[root] = "heldout" if h < int(heldout_ratio * 1000) else "train"
    return {row["question_id"]: split_of[find(row["question_id"])] for row in rows}


def build_corpus(rows: list[dict], heldout_ratio: float = 0.3, max_rows: int = 50_000) -> list[dict]:
    split = _component_split(rows, heldout_ratio)
    out: list[dict] = []
    for row in sorted(rows, key=lambda r: r["question_id"]):
        qid = row["question_id"]
        gold = set(row.get("expected_doc_ids", []))
        cands = row["candidates"]
        lexical = normalize_q16([int(c.get("lexical_score", 0)) for c in cands])
        semantic = normalize_q16([int(c.get("vector_score", 0)) for c in cands])
        recency = normalize_q16([int(c.get("recency_score", 0)) for c in cands])
        trust = normalize_q16([int(c.get("trust_score", 0)) for c in cands])
        for i, cand in enumerate(sorted(cands, key=lambda c: c["document_id"])):
            # re-index after the sort so features line up with the sorted order
            j = cands.index(cand)
            out.append({
                "split": split[qid],
                "question_id": qid,
                "question_type": row.get("question_type", "unknown"),
                "document_id": cand["document_id"],
                "features_q16": {
                    "lexical": lexical[j],
                    "semantic": semantic[j],
                    "recency": recency[j],
                    "trust": trust[j],
                },
                "label": 1 if cand["document_id"] in gold else 0,
            })
    out.sort(key=lambda r: (r["split"], r["question_id"], r["document_id"]))
    return out[:max_rows]


def manifest_hash(corpus: list[dict]) -> str:
    payload = "\n".join(json.dumps(r, sort_keys=True) for r in corpus)
    return hashlib.sha256(payload.encode()).hexdigest()


def _synthetic_rows() -> list[dict]:
    rows = []
    for i in range(60):
        gold = f"doc_gold_{i}"
        rows.append({
            "question_id": f"q_{i:03d}",
            "question_type": ["basic", "semantic", "project"][i % 3],
            "expected_doc_ids": [gold],
            "candidates": [
                {"document_id": gold, "lexical_score": 80, "vector_score": 10 + (i % 5)},
                {"document_id": f"doc_distract_{i % 7}", "lexical_score": 20, "vector_score": 70},
            ],
        })
    return rows


def self_test() -> int:
    rows = _synthetic_rows()
    corpus = build_corpus(rows, heldout_ratio=0.3, max_rows=50_000)
    # 1. Determinism.
    assert manifest_hash(corpus) == manifest_hash(build_corpus(rows)), "build not deterministic"
    # 2. No question_id crosses splits.
    q_split: dict[str, str] = {}
    for r in corpus:
        prev = q_split.setdefault(r["question_id"], r["split"])
        assert prev == r["split"], f"question {r['question_id']} in two splits"
    # 3. No document_id crosses splits (the strict F05 requirement).
    doc_split: dict[str, str] = {}
    for r in corpus:
        prev = doc_split.setdefault(r["document_id"], r["split"])
        assert prev == r["split"], f"document {r['document_id']} leaks across the split"
    # 4. Both splits populated + positives present.
    splits = {r["split"] for r in corpus}
    positives = sum(r["label"] for r in corpus)
    assert splits == {"train", "heldout"}, f"expected both splits, got {splits}"
    assert positives >= len(rows), "each question must contribute at least one positive"
    # 5. Features are in Q16 range.
    for r in corpus:
        for v in r["features_q16"].values():
            assert 0 <= v <= Q16
    print(f"ltr-corpus self-test passed: {len(corpus)} rows, {positives} positives, "
          f"no question/document leakage, deterministic (manifest {manifest_hash(corpus)[:12]})")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--input", type=pathlib.Path)
    ap.add_argument("--output", type=pathlib.Path)
    ap.add_argument("--heldout-ratio", type=float, default=0.3)
    ap.add_argument("--max-rows", type=int, default=50_000)
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    if not args.input or not args.output:
        ap.error("--input and --output are required unless --self-test")
    rows = [json.loads(line) for line in args.input.read_text().splitlines() if line.strip()]
    corpus = build_corpus(rows, args.heldout_ratio, args.max_rows)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(json.dumps(r, sort_keys=True) for r in corpus) + "\n")
    print(f"wrote {len(corpus)} rows to {args.output} (manifest {manifest_hash(corpus)[:12]})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
