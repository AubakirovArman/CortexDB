from __future__ import annotations

import argparse
import json
from pathlib import Path

from .runner import run

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--base-retrieval-file", type=Path, required=True)
    parser.add_argument("--extra-retrieval-file", type=Path, action="append", default=[])
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-k", type=int, default=500)
    parser.add_argument("--base-limit", type=int, default=500)
    parser.add_argument("--extra-limit", type=int, default=500)
    parser.add_argument("--path-candidate-limit", type=int, default=800)
    parser.add_argument("--path-existing-only", action="store_true")
    parser.add_argument("--path-terms-mode", choices=["all", "entity"], default="all")
    parser.add_argument("--enable-path-ngrams", action="store_true")
    parser.add_argument("--content-candidate-limit", type=int, default=1200)
    parser.add_argument("--content-boost-limit", type=int, default=40)
    parser.add_argument("--content-preview-chars", type=int, default=1800)
    parser.add_argument("--content-score-threshold", type=float, default=0.0)
    parser.add_argument("--phrase-candidate-limit", type=int, default=0)
    parser.add_argument("--phrase-boost-limit", type=int, default=0)
    parser.add_argument("--phrase-max-posting", type=int, default=50000)
    parser.add_argument("--neighbor-expansion-limit", type=int, default=0)
    parser.add_argument("--neighbor-seed-limit", type=int, default=40)
    parser.add_argument("--neighbor-max-per-seed", type=int, default=6)
    parser.add_argument("--neighbor-max-posting", type=int, default=400)
    parser.add_argument("--enable-source-link-neighbors", action="store_true")
    parser.add_argument(
        "--content-existing-only-question-type",
        action="append",
        default=["constrained"],
        help="For these question types, content preview only boosts docs already found by base/extra retrieval.",
    )
    parser.add_argument("--max-posting", type=int, default=12000)
    parser.add_argument("--rrf-k", type=int, default=60)
    parser.add_argument("--weight-base-rrf", type=float, default=900.0)
    parser.add_argument("--weight-extra-rrf", type=float, default=500.0)
    parser.add_argument("--weight-path", type=float, default=1.0)
    parser.add_argument("--weight-content", type=float, default=0.01)
    parser.add_argument("--weight-phrase", type=float, default=1.0)
    parser.add_argument("--weight-neighbor", type=float, default=1.0)
    parser.add_argument("--source-match-boost", type=float, default=0.0)
    parser.add_argument("--enable-query-type-router", action="store_true")
    parser.add_argument("--diagnostics-top-k", type=int, default=5)
    args = parser.parse_args()
    for name in (
        "top_k",
        "base_limit",
        "extra_limit",
        "path_candidate_limit",
        "content_candidate_limit",
        "content_boost_limit",
        "content_preview_chars",
        "max_posting",
        "rrf_k",
        "phrase_max_posting",
        "neighbor_seed_limit",
        "neighbor_max_per_seed",
        "neighbor_max_posting",
    ):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    for name in ("phrase_candidate_limit", "phrase_boost_limit", "neighbor_expansion_limit"):
        if getattr(args, name) < 0:
            parser.error(f"--{name.replace('_', '-')} must be non-negative")
    if args.diagnostics_top_k < 0:
        parser.error("--diagnostics-top-k must be non-negative")
    return args


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "questions": report["questions"],
                "average_recall_pct": report["average_recall_pct"],
                "full_recall_questions": report["full_recall_questions"],
                "output": report["output"],
            },
            sort_keys=True,
        )
    )
    return 0

