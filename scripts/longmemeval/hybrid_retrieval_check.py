#!/usr/bin/env python3
"""A6.3 (fast/offline half): prove the LongMemEval hybrid-retrieval harness logic
without an endpoint — the keyword default stays byte-identical to the committed
F3.1 baseline, the hybrid path adds the dense `vector=` payload line + `--mode
hybrid --vector=` search, and the Q15 scaling is correct.

The metered comparison (hybrid vs the committed keyword baseline over the 500-row
split, recall_all@10 >= 0.93 narrative, <=10 per-question regressions) plugs into
the same harness via `--retrieval-mode hybrid`.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import v1_cortexdb_retrieval as h  # noqa: E402


def run_self_test() -> int:
    failures: list[str] = []
    row = {"corpus_id": "c1", "timestamp": "2023", "index_text": "hello world"}

    # (1) keyword default -> byte-identical to pre-A6.3 payload (no vector line).
    keyword = h.payload_for_cell("q1", row)
    want = "\n".join([
        "scope=longmemeval", "status=ready", "type=memory",
        "source=longmemeval:q1:c1", "",
        "LONGMEMEVAL_CORPUS_ID: c1", "LONGMEMEVAL_TIMESTAMP: 2023", "", "hello world",
    ])
    if keyword != want:
        failures.append("keyword payload not byte-identical to pre-A6.3")
    if "vector=" in keyword:
        failures.append("keyword payload leaked a vector line")

    # (2) hybrid -> adds the vector line in the header (before the corpus body).
    hybrid = h.payload_for_cell("q1", row, "1,2,3")
    if "vector=1,2,3" not in hybrid:
        failures.append("hybrid payload missing the vector line")
    elif hybrid.index("vector=") > hybrid.index("LONGMEMEVAL_CORPUS_ID"):
        failures.append("hybrid vector line must be in the header block")

    # (3) Q15 scaling: unit-normalize then scale by 32767.
    if h.q15_literal([3.0, 4.0]) != "19660,26214":
        failures.append(f"q15 scaling wrong: {h.q15_literal([3.0, 4.0])}")
    if h.q15_literal([0.0, 0.0]) != "0,0":
        failures.append("q15 zero-vector must be all zeros")

    # (4) search command construction (capture without running the binary).
    captured: dict[str, list[str]] = {}
    original = h.run_command
    h.run_command = lambda cmd, cwd=None: captured.__setitem__("cmd", cmd) or '{"results": []}'
    try:
        h.search_cortexdb(pathlib.Path("bin"), pathlib.Path("db"), "q", 10, mode="keyword")
        kw_cmd = captured["cmd"]
        if "keyword" not in kw_cmd or any(a.startswith("--vector") for a in kw_cmd):
            failures.append(f"keyword command wrong: {kw_cmd}")
        h.search_cortexdb(
            pathlib.Path("bin"), pathlib.Path("db"), "q", 10,
            mode="hybrid", query_vector_literal="-2,3",
        )
        hy_cmd = captured["cmd"]
        if "hybrid" not in hy_cmd or "--vector=-2,3" not in hy_cmd:
            failures.append(f"hybrid command must use --vector= (= form): {hy_cmd}")
    finally:
        h.run_command = original

    if failures:
        print("A6.3 hybrid-retrieval self-test FAILED:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print("A6.3 hybrid-retrieval self-test passed: keyword default byte-identical, "
          "hybrid adds the dense vector payload + --vector= search, Q15 scaling correct.")
    return 0


if __name__ == "__main__":
    raise SystemExit(run_self_test())
