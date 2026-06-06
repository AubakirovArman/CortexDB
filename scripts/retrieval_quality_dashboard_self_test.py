#!/usr/bin/env python3
"""Self-test for retrieval_quality_dashboard.py."""

from __future__ import annotations

import sys

from retrieval_quality_dashboard import render_dashboard


def sample_beta() -> dict[str, object]:
    return {
        "status": "passed",
        "production_safe": True,
        "domain_count": 1,
        "repeat_runs_per_domain": 2,
        "top_k": 10,
        "boundary": {
            "proves": "local evidence",
            "does_not_prove": "private relevance",
        },
        "domains": [
            {
                "domain": "demo",
                "documents": 2,
                "chunks": 4,
                "queries": 2,
                "latest_mean_recall_q16": 65535,
                "latest_mean_mrr_q16": 32767,
                "latest_mean_ndcg_q16": 49151,
                "latest_p95_latency_nanos": 2000,
                "latest_exact_parity_q16": 65535,
                "regression_count": 0,
            }
        ],
    }


def sample_report() -> dict[str, object]:
    return {
        "modes": {
            "guarded_ann": {
                "mean_recall_q16": 65535,
                "mean_mrr_q16": 32767,
                "mean_ndcg_q16": 49151,
                "p95_latency_nanos": 2000,
                "exact_parity_q16": 65535,
            }
        },
        "query_level": [
            {
                "name": "q1",
                "recall_q16": 65535,
                "mrr_q16": 65535,
                "ndcg_q16": 65535,
                "latency_nanos": 1500,
                "exact_parity": True,
                "production_safe": True,
            }
        ],
    }


def sample_history() -> dict[str, object]:
    return {
        "domains": [
            {
                "domain": "demo",
                "latest_run_id": "demo-history-002",
                "latest_p95_latency_nanos": 2000,
            }
        ],
        "runs": [
            {"domain": "demo", "run_id": "demo-history-001", "p95_latency_nanos": 2500},
            {"domain": "demo", "run_id": "demo-history-002", "p95_latency_nanos": 2000},
        ],
    }


def main() -> int:
    html = render_dashboard(sample_report(), sample_beta(), sample_history())
    required = [
        "Recall Panel",
        "MRR Panel",
        "nDCG Panel",
        "Latency Trend Panel",
        "Domain Quality Table",
        "Query-Level Table",
        "p95 latency",
        "faster",
    ]
    missing = [marker for marker in required if marker not in html]
    if missing:
        print(f"retrieval dashboard self-test failed: missing {missing}", file=sys.stderr)
        return 1
    print("retrieval dashboard self-test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
