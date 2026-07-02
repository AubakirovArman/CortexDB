#!/usr/bin/env python3
"""Validate the HNSW cosine metric correctness gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_METRIC_TERMS = [
    "cosine_similarity_q16",
    "DistanceMetric::Cosine",
    "Some(u64::from(cosine_similarity_q16(u, v)))",
]

REQUIRED_HELPER_TERMS = [
    "dot_product <= 0",
    "saturating_mul(u128::from(u16::MAX) + 1)",
    "min(u128::from(u16::MAX))",
]

REQUIRED_TEST_TERMS = [
    "cosine_metric_rejects_anticorrelated_vectors",
    "cosine_metric_orders_positive_before_orthogonal_and_negative",
    "cosine_metric_handles_high_dimensional_vectors_without_overflow",
    "cosine_metric_matches_shared_q16_helper",
]

REQUIRED_MAKE_TERMS = [
    "cosine-metric-correctness-check:",
    "hnsw-cosine-correctness-check:",
]

FORBIDDEN_METRIC_TERMS = [
    "dot.abs()",
    ".abs() * 65_535",
]


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden {term}" for term in terms if term in text]


def validate(root: Path) -> dict[str, Any]:
    metric = read_text(root / "crates/cortex-engine/src/search/hnsw/metric.rs")
    helper = read_text(root / "crates/cortex-engine/src/search/vector_similarity.rs")
    dedup = read_text(root / "crates/cortex-engine/src/context/dedup.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )

    failures: list[str] = []
    failures.extend(missing_terms("metric.rs", metric, REQUIRED_METRIC_TERMS))
    failures.extend(missing_terms("vector_similarity.rs", helper, REQUIRED_HELPER_TERMS))
    failures.extend(missing_terms("metric.rs tests", metric, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(forbidden_terms("metric.rs", metric, FORBIDDEN_METRIC_TERMS))
    if "use crate::search::vector_similarity::cosine_similarity_q16;" not in dedup:
        failures.append("context/dedup.rs: must use shared cosine_similarity_q16 helper")

    return {
        "schema_version": "cortexdb.cosine_metric_correctness.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "metric_terms": REQUIRED_METRIC_TERMS,
            "helper_terms": REQUIRED_HELPER_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_metric_terms": FORBIDDEN_METRIC_TERMS,
        },
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"cosine metric correctness check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
