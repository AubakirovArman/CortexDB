#!/usr/bin/env python3
"""F3.4 (QA half): reshape a LoCoMo retrieval log into run_qa.py's input schema.

The `locomo-cortexdb-retrieval` log already carries the retrieved dialogue turn
text (each `retrieval_list` item has speaker/text/session/date), so this is a
pure reshape — no corpus hydration needed. It maps LoCoMo's numeric category
codes to the names run_qa.py branches on and emits, per question:

    {question_id, category, question, answer, retrieved_turns:[{speaker,text,timestamp}]}

Two entry points (the F2.2/F3.1 self-tested-harness pattern):
  - FAST (`--self-test`): reshape a tiny committed retrieval fixture and assert
    the schema + category mapping + turn text carry-through, offline.
  - REAL: `--retrieval-log <cortexdb_locomo_retrieval.jsonl> --output <qa_input.jsonl>`
    then feed --output to run_qa.py; optionally `--limit N` (repo rule: 50 first).

Dependency-free (stdlib only); deterministic.
"""
from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
FIX = REPO / "fixtures" / "benchmarks" / "locomo"
SELFTEST_LOG = FIX / "qa_retrieval_sample.jsonl"

# LoCoMo official category codes -> the names run_qa.py's CATEGORY_INSTRUCTIONS
# branch on (snap-research/locomo).
CATEGORY = {1: "multi-hop", 2: "temporal-reasoning", 3: "open-domain", 4: "single-hop", 5: "adversarial"}


def read_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def reshape(rows: list[dict], topk: int = 10) -> list[dict]:
    out = []
    for row in rows:
        turns = [
            {"speaker": t.get("speaker", ""), "text": t.get("text", ""), "timestamp": t.get("date", "")}
            for t in row.get("retrieval_list", [])[:topk]
        ]
        out.append({
            "question_id": row["question_id"],
            "category": CATEGORY.get(row.get("category"), "open-domain"),
            "question": row["question"],
            "answer": str(row.get("answer", "")),
            "retrieved_turns": turns,
        })
    return out


def balanced_subset(rows: list[dict], limit: int) -> list[dict]:
    """Deterministic round-robin across categories (stable order)."""
    by = collections.defaultdict(list)
    for r in rows:
        by[r["category"]].append(r)
    cats = sorted(by)
    out: list[dict] = []
    i = 0
    while len(out) < limit and any(by.values()):
        c = cats[i % len(cats)]
        if by[c]:
            out.append(by[c].pop(0))
        i += 1
    return out


def run_self_test() -> int:
    rows = read_jsonl(SELFTEST_LOG)
    shaped = reshape(rows)
    failures = []
    if len(shaped) != len(rows):
        failures.append(f"reshaped {len(shaped)} for {len(rows)} rows")
    for src, dst in zip(rows, shaped):
        for field in ("question_id", "category", "question", "answer", "retrieved_turns"):
            if field not in dst:
                failures.append(f"{dst.get('question_id')}: missing {field}")
        if dst["category"] != CATEGORY.get(src.get("category")):
            failures.append(f"{dst['question_id']}: category {src.get('category')!r} mapped wrong")
        want = src.get("retrieval_list", [])[:10]
        if len(dst["retrieved_turns"]) != len(want):
            failures.append(f"{dst['question_id']}: dropped turns")
        if want and dst["retrieved_turns"][0]["text"] != want[0].get("text", ""):
            failures.append(f"{dst['question_id']}: turn text not carried through")
    seen = {d["category"] for d in shaped}
    if not seen <= set(CATEGORY.values()):
        failures.append(f"unknown categories: {seen - set(CATEGORY.values())}")
    if failures:
        print("F3.4 LoCoMo QA-input adapter self-test FAILED:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(
        f"F3.4 LoCoMo QA-input adapter self-test passed: {len(shaped)} rows reshaped, "
        f"categories {sorted(seen)}, turn text carried through."
    )
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--retrieval-log")
    ap.add_argument("--output")
    ap.add_argument("--topk", type=int, default=10)
    ap.add_argument("--limit", type=int, default=None, help="balanced round-robin subset")
    args = ap.parse_args(argv)
    if args.self_test:
        return run_self_test()
    if not args.retrieval_log or not args.output:
        ap.error("real reshape needs --retrieval-log --output (or pass --self-test)")
    rows = read_jsonl(pathlib.Path(args.retrieval_log))
    shaped = reshape(rows, topk=args.topk)
    if args.limit is not None:
        shaped = balanced_subset(shaped, args.limit)
    out = pathlib.Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(json.dumps(r) for r in shaped) + "\n", encoding="utf-8")
    dist = dict(collections.Counter(r["category"] for r in shaped))
    print(f"wrote {len(shaped)} QA inputs -> {out} (categories: {dist})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
