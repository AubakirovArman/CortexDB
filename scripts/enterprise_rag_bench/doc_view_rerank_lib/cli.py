from __future__ import annotations

import argparse
import json
from pathlib import Path

from .runner import run

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--doc-views-file", type=Path)
    parser.add_argument("--embedding-cache", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="doc_view_rerank_v1")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--score-candidate-limit", type=int, default=140)
    parser.add_argument("--seed-count", type=int, default=4)
    parser.add_argument("--protect-baseline-prefix", type=int, default=8)
    parser.add_argument("--route-question-types", default="")
    parser.add_argument("--route-source-types", default="")
    parser.add_argument("--raw-tail-question-types", default="")
    parser.add_argument("--raw-candidate-tail-slots", type=int, default=0)
    parser.add_argument("--raw-candidate-tail-rank-limit", type=int, default=50)
    parser.add_argument("--diagnostics-top-k", type=int, default=5)
    args = parser.parse_args()
    for name in ("limit", "score_candidate_limit"):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.seed_count < 0:
        parser.error("--seed-count must be non-negative")
    if args.protect_baseline_prefix < 0:
        parser.error("--protect-baseline-prefix must be non-negative")
    if args.raw_candidate_tail_slots < 0:
        parser.error("--raw-candidate-tail-slots must be non-negative")
    if args.raw_candidate_tail_rank_limit <= 0:
        parser.error("--raw-candidate-tail-rank-limit must be positive")
    if args.diagnostics_top_k < 0:
        parser.error("--diagnostics-top-k must be non-negative")
    args.route_question_types = {
        value.strip()
        for value in args.route_question_types.split(",")
        if value.strip()
    }
    args.route_source_types = {
        value.strip()
        for value in args.route_source_types.split(",")
        if value.strip()
    }
    args.raw_tail_question_types = {
        value.strip()
        for value in args.raw_tail_question_types.split(",")
        if value.strip()
    }
    return args


def main() -> int:
    summary = run(parse_args())
    print(
        json.dumps(
            {
                "questions": summary["questions"],
                "average_recall_pct": summary["average_recall_pct"],
                "full_recall_questions": summary["full_recall_questions"],
                "changed_rows": summary["changed_rows"],
                "output": summary["output"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
