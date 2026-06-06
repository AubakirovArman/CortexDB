#!/usr/bin/env python3
"""Self-test for retrieval_quality_history.py."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

from retrieval_quality_history import build_report, compare, parse_args


def write_domain(root: Path, name: str) -> None:
    domain = root / name
    (domain / "corpus").mkdir(parents=True)
    (domain / "queries").mkdir()
    (domain / "corpus" / "chunks.jsonl").write_text(
        json.dumps({"chunk_id": f"{name}_c1", "text": f"{name} alpha beta"}) + "\n",
        encoding="utf-8",
    )
    (domain / "queries" / "queries.jsonl").write_text(
        json.dumps({"query_id": "q1", "query": "alpha beta"}) + "\n",
        encoding="utf-8",
    )
    (domain / "queries" / "ground_truth.jsonl").write_text(
        json.dumps({"query_id": "q1", "relevant_chunk_ids": [f"{name}_c1"]}) + "\n",
        encoding="utf-8",
    )


def sample_run(run_id: str, recall: int) -> dict[str, object]:
    return {
        "domain": "d",
        "run_id": run_id,
        "mean_recall_q16": recall,
        "mean_mrr_q16": 10,
        "mean_ndcg_q16": 10,
        "p95_latency_nanos": 1,
        "p99_latency_nanos": 1,
        "max_latency_nanos": 1,
        "top_by_query": {"q": ["c"]},
        "run_index": 1,
    }


def main() -> int:
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        write_domain(root, "a")
        write_domain(root, "b")
        args = parse_args(["--domain-root", str(root), "--min-domains", "2", "--history-runs", "2"])
        report = build_report(args)
        if report["status"] != "passed" or report["run_count"] != 4:
            print("retrieval quality history self-test failed: pass path", file=sys.stderr)
            return 1
    args = parse_args(["--output", "unused", "--fail-on-regression"])
    current = dict(sample_run("a", 10), run_id="b", mean_recall_q16=9, run_index=2)
    if not compare(sample_run("a", 10), current, args):
        print("retrieval quality history self-test failed: regression path", file=sys.stderr)
        return 1
    print("retrieval quality history self-test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
