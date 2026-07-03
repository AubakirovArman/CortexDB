#!/usr/bin/env python3
"""A1.3 vector-metric allowlist: the cosine similarity metric must have exactly
ONE implementation, in crates/cortex-engine/src/search/vector_similarity.rs.

A governed context engine ranks by vector similarity; if two code paths compute
cosine differently, HNSW / exact / persisted retrieval can disagree and a signed
receipt's order becomes path-dependent. This guard fails if the cosine
normalization primitives (or the `cosine_similarity_q16` definition) appear
anywhere outside the single source of truth. Callers that *use*
`cosine_similarity_q16` are fine — only re-implementations are rejected.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ENGINE_SRC = REPO / "crates" / "cortex-engine" / "src"
SOURCE_OF_TRUTH = ENGINE_SRC / "search" / "vector_similarity.rs"

# Markers that indicate a cosine-normalization implementation (not a mere call).
IMPL_MARKERS = [
    r"norm_u_sq",
    r"norm_v_sq",
    r"norm_product",
    r"fn integer_sqrt",
    r"fn cosine_similarity_q16",
]


def main() -> int:
    report_path = None
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])

    pattern = re.compile("|".join(IMPL_MARKERS))
    offenders: list[str] = []
    for rs in ENGINE_SRC.rglob("*.rs"):
        if rs.resolve() == SOURCE_OF_TRUTH.resolve():
            continue
        text = rs.read_text(encoding="utf-8", errors="ignore")
        for lineno, line in enumerate(text.splitlines(), 1):
            if pattern.search(line):
                offenders.append(f"{rs.relative_to(REPO)}:{lineno}: {line.strip()[:100]}")

    passed = not offenders and SOURCE_OF_TRUTH.exists()
    report = {
        "schema_version": "cortexdb.vector_metric_allowlist.report.v1",
        "status": "passed" if passed else "failed",
        "source_of_truth": str(SOURCE_OF_TRUTH.relative_to(REPO)),
        "offenders": offenders,
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if not passed:
        print("vector-metric allowlist FAILED: cosine reimplemented outside vector_similarity.rs")
        for o in offenders:
            print("  " + o)
        return 1
    print("vector-metric allowlist passed: cosine has one implementation (vector_similarity.rs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
