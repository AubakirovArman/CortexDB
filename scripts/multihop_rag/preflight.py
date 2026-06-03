#!/usr/bin/env python3
"""Validate official MultiHop-RAG inputs for local CortexDB runs."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def read_list(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, list):
        raise ValueError(f"{path}: expected JSON list")
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(value):
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{index}: expected object")
        rows.append(row)
    return rows


def validate_queries(path: Path) -> dict[str, Any]:
    rows = read_list(path)
    by_type: Counter[str] = Counter()
    evidence_counts: Counter[int] = Counter()
    failures: list[str] = []
    for index, row in enumerate(rows):
        label = f"{path}:{index}"
        query = row.get("query")
        question_type = row.get("question_type")
        evidence_list = row.get("evidence_list")
        if not isinstance(query, str) or not query.strip():
            failures.append(f"{label}: query must be non-empty")
        if not isinstance(question_type, str) or not question_type.strip():
            failures.append(f"{label}: question_type must be non-empty")
            question_type = "unknown"
        if not isinstance(evidence_list, list):
            failures.append(f"{label}: evidence_list must be a list")
            evidence_list = []
        by_type[str(question_type)] += 1
        evidence_counts[len(evidence_list)] += 1
        for evidence_index, evidence in enumerate(evidence_list):
            if not isinstance(evidence, dict):
                failures.append(f"{label}: evidence {evidence_index} must be an object")
                continue
            if not isinstance(evidence.get("fact"), str) or not evidence["fact"].strip():
                failures.append(f"{label}: evidence {evidence_index} missing fact")
    return {
        "rows": len(rows),
        "by_question_type": dict(sorted(by_type.items())),
        "evidence_counts": {str(key): value for key, value in sorted(evidence_counts.items())},
        "failures": failures,
    }


def validate_corpus(path: Path) -> dict[str, Any]:
    rows = read_list(path)
    failures: list[str] = []
    sources: Counter[str] = Counter()
    for index, row in enumerate(rows):
        label = f"{path}:{index}"
        body = row.get("body")
        if not isinstance(body, str) or not body.strip():
            failures.append(f"{label}: body must be non-empty")
        source = row.get("source")
        if isinstance(source, str) and source.strip():
            sources[source] += 1
        else:
            failures.append(f"{label}: source must be non-empty")
        for field in ["title", "published_at", "url"]:
            if not isinstance(row.get(field), str) or not row[field].strip():
                failures.append(f"{label}: {field} must be non-empty")
    return {"rows": len(rows), "source_count": len(sources), "failures": failures}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--queries", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()

    query_report = validate_queries(args.queries)
    corpus_report = validate_corpus(args.corpus)
    failures = query_report["failures"] + corpus_report["failures"]
    report = {
        "schema_version": "cortexdb.multihop_rag.preflight_report.v1",
        "status": "failed" if failures else "passed",
        "queries": query_report,
        "corpus": corpus_report,
        "failures": failures,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"MultiHop-RAG preflight failed: {args.report}")
        for failure in failures[:20]:
            print(f"- {failure}")
        return 1
    print(f"MultiHop-RAG preflight passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
