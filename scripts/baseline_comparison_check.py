#!/usr/bin/env python3
"""Compare CortexDB evidence with a small naive retrieval stack.

The naive stack intentionally has no repo dependencies: SQLite FTS5 for lexical
search, deterministic exact hashed vectors for the dense slot, and RRF for the
hybrid result. This keeps the C20 comparison reproducible in CI while making
the dense side explicit instead of pretending that FAISS is available.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from baseline_comparison_common import write_json
from baseline_comparison_report import build_markdown, build_report
from baseline_comparison_self_test import run_self_test


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--datasets", type=Path, default=Path("fixtures/context_pack_quality_v3_datasets.json"))
    parser.add_argument("--features", type=Path, default=Path("fixtures/baseline_comparison/feature_matrix.json"))
    parser.add_argument("--cortexdb-retrieval-report", type=Path, default=Path("target/retrieval-quality/beta-report.json"))
    parser.add_argument("--context-pack-report", type=Path, default=Path("target/context-pack-quality/v3-report.json"))
    parser.add_argument("--report", type=Path, default=Path("target/baseline-comparison/report.json"))
    parser.add_argument("--markdown", type=Path, default=Path("docs/BASELINE_COMPARISON.md"))
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--repeat-runs", type=int, default=3)
    parser.add_argument("--min-domains", type=int, default=4)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        try:
            run_self_test()
        except (OSError, ValueError, json.JSONDecodeError, AssertionError, RuntimeError) as error:
            print(f"baseline comparison self-test failed: {error}", file=sys.stderr)
            return 1
        print("baseline comparison self-test passed")
        return 0
    try:
        report = build_report(args)
    except (OSError, ValueError, json.JSONDecodeError, RuntimeError) as error:
        print(f"baseline comparison failed: {error}", file=sys.stderr)
        return 1
    write_json(args.report, report)
    args.markdown.parent.mkdir(parents=True, exist_ok=True)
    args.markdown.write_text(build_markdown(report), encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"baseline comparison passed: {args.report} {args.markdown}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
